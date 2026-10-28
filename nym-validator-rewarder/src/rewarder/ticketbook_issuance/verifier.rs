// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::rewarder::ticketbook_issuance::types::{
    CredentialIssuer, OperatorIssuing, TicketbookIssuanceResults,
};
use cosmwasm_std::Decimal;
use nym_compact_ecash::scheme::withdrawal::verify_partial_blind_signature;
use nym_compact_ecash::{CompactEcashError, date_scalar, type_scalar};
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::EcashTime;
use nym_network_defaults::MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE;
use nym_ticketbooks_merkle::{IssuedTicketbook, MerkleLeaf};
use nym_validator_client::ecash::models::{
    CommitedDeposit, DepositId, IssuedTicketbooksChallengeCommitmentRequestBody,
    IssuedTicketbooksChallengeCommitmentResponse, IssuedTicketbooksDataRequestBody,
    IssuedTicketbooksDataResponse, IssuedTicketbooksDataResponseBody, IssuedTicketbooksForResponse,
};
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::signable::{SignableMessageBody, SignedMessage};
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::any::type_name_of_val;
use std::cmp::max;
use std::collections::{BTreeMap, HashMap, HashSet};
use thiserror::Error;
use time::Date;
use tracing::{debug, error, info, instrument, warn};

#[derive(Error, Debug)]
enum PartialTicketbookVerificationFailure {
    #[error("failed to deserialise associated blinded signature: {0}")]
    MalformedBlindedSignature(CompactEcashError),

    #[error("failed to deserialise private attributes commitments: {0}")]
    MalformedPrivateAttributesCommitments(CompactEcashError),

    #[error("the associated blinded signature failed to get verified")]
    InvalidSignature,
}

#[derive(Serialize, Deserialize)]
pub struct Empty {}

#[derive(Serialize, Deserialize)]
pub struct MismatchResponse<T, R> {
    requested: T,
    received: T,
    signed_response: R,
}

#[derive(Serialize, Deserialize)]
pub struct TamperedOriginalRequest<T> {
    // internally it will have a rather obvious field indicating the original (signed) request
    signed_response: T,
}

#[derive(Serialize, Deserialize)]
pub struct MismatchClaim<T> {
    claimed: T,
    actual: T,
}

#[derive(Serialize, Deserialize)]
pub struct GenericError {
    error: String,
}

pub struct IssuerBan {
    pub reason: String,
    pub serialised_evidence: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct CheatingEvidence<T = Empty> {
    rewarder_pubkey: ed25519::PublicKey,
    issuer_pubkey: ed25519::PublicKey,
    commitment: Option<IssuedTicketbooksForResponse>,
    requested_challenge: Vec<DepositId>,
    challenge_commitment: Option<IssuedTicketbooksChallengeCommitmentResponse>,
    ticketbook_data_responses: Vec<IssuedTicketbooksDataResponse>,

    #[serde(flatten)]
    inner: T,
}

pub struct IssuerUnderTest<C = nym_http_api_client::Client> {
    pub(crate) rewarder_pubkey: ed25519::PublicKey,
    pub(crate) details: CredentialIssuer<C>,
    pub(crate) verification_skipped: bool,
    pub(crate) issuer_ban: Option<IssuerBan>,
    pub(crate) issued_commitment: Option<IssuedTicketbooksForResponse>,
    pub(crate) sampled_deposits: HashMap<DepositId, CommitedDeposit>,
    pub(crate) challenge_commitment_response: Option<IssuedTicketbooksChallengeCommitmentResponse>,
    pub(crate) ticketbook_data_responses: Vec<IssuedTicketbooksDataResponse>,
}

impl<C: NymApiClientExt + Sync> IssuerUnderTest<C> {
    fn new(details: CredentialIssuer<C>, rewarder_pubkey: ed25519::PublicKey) -> Self {
        IssuerUnderTest {
            rewarder_pubkey,
            details,
            verification_skipped: false,
            issuer_ban: None,
            issued_commitment: None,
            sampled_deposits: HashMap::new(),
            challenge_commitment_response: None,
            ticketbook_data_responses: vec![],
        }
    }

    fn issued_merkle_root_commitment(&self) -> Option<[u8; 32]> {
        self.issued_commitment
            .as_ref()
            .and_then(|i| i.body.merkle_root)
    }

    fn max_data_request_size(&self) -> Option<usize> {
        self.challenge_commitment_response
            .as_ref()
            .map(|r| r.body.max_data_response_size)
    }

    fn caught_cheating(&self) -> bool {
        self.issuer_ban.is_some()
    }

    fn produce_basic_cheating_evidence(&self) -> CheatingEvidence {
        self.produce_cheating_evidence(Empty {})
    }

    fn produce_generic_cheating_evidence<S: Into<String>>(
        &self,
        error: S,
    ) -> CheatingEvidence<GenericError> {
        self.produce_cheating_evidence(GenericError {
            error: error.into(),
        })
    }

    fn produce_cheating_evidence<T>(&self, additional_context: T) -> CheatingEvidence<T> {
        CheatingEvidence {
            rewarder_pubkey: self.rewarder_pubkey,
            issuer_pubkey: self.details.public_key,
            commitment: self.issued_commitment.clone(),
            requested_challenge: self.sampled_deposits.keys().copied().collect(),
            challenge_commitment: self.challenge_commitment_response.clone(),
            ticketbook_data_responses: self.ticketbook_data_responses.clone(),
            inner: additional_context,
        }
    }

    // SAFETY: we're using stable serialisation
    #[allow(clippy::unwrap_used)]
    fn set_banned_issuer<T>(&mut self, reason: impl Into<String>, evidence: CheatingEvidence<T>)
    where
        T: Serialize,
    {
        let reason = reason.into();
        warn!(
            "[CHEATING] banning {} for cheating because of: {reason}",
            self.details
        );
        self.issuer_ban = Some(IssuerBan {
            reason,
            serialised_evidence: serde_json::to_vec(&evidence).unwrap(),
        })
    }

    // returns bool to indicate if the issuer got banned
    fn ban_if_tampered_request<T>(&mut self, original_request: &SignedMessage<T>) -> bool
    where
        T: SignableMessageBody,
    {
        if !original_request.verify_signature(&self.rewarder_pubkey) {
            error!(
                "❗ {} HAS TAMPERED WITH THE {} REQUEST ❗",
                self.details,
                type_name_of_val(original_request)
            );
            let evidence = self.produce_cheating_evidence(TamperedOriginalRequest {
                signed_response: original_request,
            });
            self.set_banned_issuer("original request body was tampered with", evidence);
            return true;
        }
        false
    }

    async fn get_ticketbooks_data(
        &mut self,
        signing_key: &ed25519::PrivateKey,
        expiration_date: Date,
    ) {
        debug!("getting issued ticketbooks data");

        // no point in continuing
        if self.caught_cheating() {
            return;
        }

        // nothing to get data on
        if self.sampled_deposits.is_empty() {
            return;
        }

        let Some(batch_size) = self.max_data_request_size() else {
            return;
        };

        let sampled = self.sampled_deposits.keys().copied().collect::<Vec<_>>();

        let batches = sampled.chunks(batch_size).collect::<Vec<_>>();
        let num_batches = batches.len();

        for (i, batch) in batches.into_iter().enumerate() {
            debug!(
                "batch {}/{num_batches} for getting ticketbooks data from {}...",
                i + 1,
                self.details
            );
            // we have to sign the request so that the receiver couldn't claim we requested something else
            // when the response doesn't return expected data
            let request = IssuedTicketbooksDataRequestBody::new(expiration_date, batch.to_vec())
                .sign(signing_key);
            let data_response = match self
                .details
                .api_client
                .issued_ticketbooks_data(&request)
                .await
            {
                Ok(res) => res,
                Err(err) => {
                    error!("❗ RESPONSE FAILURE ❗");
                    // they can't fail to respond now. what if they received "unfavourable" deposit id?
                    // we have to assume they're cheating
                    let evidence = self.produce_cheating_evidence(GenericError {
                        error: err.to_string(),
                    });
                    self.set_banned_issuer(
                        format!("no response for issued ticketbook data for {expiration_date} that included deposits {batch:?}"),
                        evidence,
                    );
                    return;
                }
            };

            // 1. check if the signature on the response matches
            if !data_response.verify_signature(&self.details.public_key) {
                error!("❗ RESPONSE SIGNATURE MISMATCH ❗");
                let evidence = self.produce_basic_cheating_evidence();
                self.set_banned_issuer(
                    format!("bad signature on the data response for {expiration_date} that included deposits {batch:?} "),
                    evidence,
                );
                return;
            }

            // 2. check if the signature on original request still matches
            if self.ban_if_tampered_request(&data_response.body.original_request) {
                return;
            }

            // 3. make sure every requested deposit is in the response
            if batch.len() != data_response.body.partial_ticketbooks.len() {
                error!("❗ TICKETBOOKS DATA LENGTH MISMATCH ❗");
                let res_len = data_response.body.partial_ticketbooks.len();
                let evidence = self.produce_cheating_evidence(data_response);
                self.set_banned_issuer(
                    format!(
                        "incomplete response - requested {} deposits but got {res_len} back",
                        batch.len(),
                    ),
                    evidence,
                );
                return;
            }
            for deposit_id in batch {
                if !data_response
                    .body
                    .partial_ticketbooks
                    .contains_key(deposit_id)
                {
                    error!("❗ TICKETBOOKS DATA MISMATCH - MISSING DEPOSIT {deposit_id} ❗");
                    let evidence = self.produce_cheating_evidence(data_response);
                    self.set_banned_issuer(
                        format!("incomplete response - {deposit_id} is missing"),
                        evidence,
                    );
                    return;
                }
            }

            // 4. append results to the total
            info!(
                "✅ obtained ticketbooks data batch {} / {num_batches} ({} values)",
                i + 1,
                data_response.body.partial_ticketbooks.len()
            );
            self.ticketbook_data_responses.push(data_response);
        }
    }

    async fn get_issued_count(&self, expiration_date: Date) -> usize {
        match self
            .details
            .api_client
            .issued_ticketbooks_for_count(expiration_date)
            .await
        {
            Ok(res) => {
                info!(
                    "✅ {} claims to have issued {} ticketbooks with expiration on {expiration_date}",
                    self.details, res.total
                );
                res.total
            }
            Err(err) => {
                warn!(
                    "⚠️ {} does not support queries required for determining issuance rewards: {err}",
                    self.details
                );
                0
            }
        }
    }

    async fn get_issued_commitment(&mut self, expiration_date: Date) {
        debug!("getting issued ticketbooks information");
        let issued_ticketbooks = match self
            .details
            .api_client
            .issued_ticketbooks_for(expiration_date)
            .await
        {
            Ok(res) => res,
            Err(err) => {
                warn!(
                    "⚠️ failed to obtain issued ticketbooks information from {}. it might be running an outdated api. the error was: {err}",
                    self.details
                );
                return;
            }
        };

        self.issued_commitment = Some(issued_ticketbooks.clone());

        // 1. check if the signature on the response matches
        if !issued_ticketbooks.verify_signature(&self.details.public_key) {
            error!("❗ RESPONSE SIGNATURE MISMATCH ❗");
            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!("bad signature on the issued ticketbooks for {expiration_date}"),
                evidence,
            );
            return;
        }

        // a commitment for another date proves nothing about this one. it is not punished, since
        // nothing was proven either way, but nothing is retained from it and so nothing is earned
        if expiration_date != issued_ticketbooks.body.expiration_date {
            warn!(
                "❗ EXPIRATION DATE MISMATCH ❗ requested {expiration_date}, got {}",
                issued_ticketbooks.body.expiration_date
            );
            self.issued_commitment = None;
            return;
        }

        // the root is what binds the deposit list: deposits committed without one prove nothing and
        // cannot be challenged. this is a property of the commitment alone, so it is banned here,
        // before the sampling coin toss, rather than only along the full-challenge path
        if !issued_ticketbooks.body.deposits.is_empty()
            && issued_ticketbooks.body.merkle_root.is_none()
        {
            error!("❗ EMPTY MERKLE ROOT ❗");
            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!(
                    "no merkle root for {expiration_date} despite {} committed deposits",
                    issued_ticketbooks.body.deposits.len()
                ),
                evidence,
            );
            return;
        }

        info!(
            "✅ obtained issued ticketbooks commitment for {} ticketbooks: {:?}",
            issued_ticketbooks.body.deposits.len(),
            issued_ticketbooks
                .body
                .merkle_root_hex()
                .unwrap_or_default()
        );
    }

    async fn issue_deposit_challenge(
        &mut self,
        signing_key: &ed25519::PrivateKey,
        expiration_date: Date,
    ) {
        debug!("getting issued ticketbooks challenge commitment");

        // no point in continuing
        if self.caught_cheating() {
            return;
        }

        // nothing to challenge on
        if self.sampled_deposits.is_empty() {
            return;
        }

        // a rootless commitment is banned in `get_issued_commitment`, so a sampled, still-unbanned
        // issuer always has a committed root by the time we reach the challenge
        let Some(merkle_root) = self.issued_merkle_root_commitment() else {
            error!("reached the deposit challenge without a committed merkle root");
            return;
        };

        // the merkle proof only verifies when its leaves are sorted by index, and the signer builds
        // the proof in the order we request; so ask for the deposits in merkle-index order
        let mut sampled_deposits = self.sampled_deposits.values().collect::<Vec<_>>();
        sampled_deposits.sort_by_key(|d| d.merkle_index);
        let sampled = sampled_deposits
            .into_iter()
            .map(|d| d.deposit_id)
            .collect::<Vec<_>>();

        debug!("sampled deposits: {sampled:?}",);

        // 1. get the response
        let request =
            IssuedTicketbooksChallengeCommitmentRequestBody::new(expiration_date, sampled.clone())
                .sign(signing_key);
        let challenge_commitment = match self
            .details
            .api_client
            .issued_ticketbooks_challenge_commitment(&request)
            .await
        {
            Ok(res) => res,
            Err(err) => {
                error!("❗ RESPONSE FAILURE ❗");
                // they can't fail to respond now. what if they received "unfavourable" deposit id?
                // we have to assume they're cheating
                let evidence = self.produce_generic_cheating_evidence(err.to_string());
                self.set_banned_issuer(
                    format!("no response for issued ticketbook challenge commitment for {expiration_date}"),
                    evidence,
                );
                return;
            }
        };

        self.challenge_commitment_response = Some(challenge_commitment.clone());

        // 2. check if the signature on the response matches
        if !challenge_commitment.verify_signature(&self.details.public_key) {
            error!("❗ RESPONSE SIGNATURE MISMATCH ❗");
            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!("bad signature on challenge commitment for {expiration_date}"),
                evidence,
            );
            return;
        }

        // 3. check if their reported max batch size is not pathetically small and below bare minimum (nym api would fail to start with that)
        // if that's the case they're clearly messing around
        if challenge_commitment.body.max_data_response_size < MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE {
            error!("❗ ILLEGAL MAX REQUEST SIZE ❗");
            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!(
                    "max data request size below minimum of {MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE}"
                ),
                evidence,
            );
            return;
        }

        // 4. check if the signature on original request still matches
        if self.ban_if_tampered_request(&challenge_commitment.body.original_request) {
            return;
        }

        // 5. verify whether the expiration date matches the requested value
        if expiration_date != challenge_commitment.body.expiration_date {
            error!("❗ EXPIRATION DATE MISMATCH ❗");
            let evidence = self.produce_cheating_evidence(MismatchResponse {
                requested: expiration_date,
                received: challenge_commitment.body.expiration_date,
                signed_response: challenge_commitment,
            });
            self.set_banned_issuer(
                format!("invalid deposits challenge commitment response for {expiration_date}"),
                evidence,
            );
            return;
        }

        let merkle_proof = &challenge_commitment.body.merkle_proof;
        // 6.1 perform verification of the provided proof itself
        // (if it's invalid, there's no point in getting full data)
        if !merkle_proof.verify(merkle_root) {
            error!("❗ MERKLE PROOF FAILURE ❗");

            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!("invalid merkle proof for {expiration_date}"),
                evidence,
            );
            return;
        }

        // 6.2. the proof must be over the very tree the issuer committed to, so its leaf count
        // is the number of committed deposits, not the number we happened to sample
        let committed = self.claimed_issued();
        if merkle_proof.total_leaves() != committed {
            error!("❗ MERKLE PROOF LEAVES MISMATCH ❗");

            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!(
                    "invalid merkle proof for {expiration_date} - the proof is over {} leaves whilst {committed} deposits were committed to",
                    merkle_proof.total_leaves()
                ),
                evidence,
            );
            return;
        }

        info!("✅ obtained issued ticketbooks challenge commitment");
    }

    fn verify_partial_ticketbook(
        &self,
        partial_ticketbook: &IssuedTicketbook,
    ) -> Result<(), PartialTicketbookVerificationFailure> {
        let blinded_sig =
            match IssuedTicketbooksDataResponseBody::try_get_partial_credential(partial_ticketbook)
            {
                Ok(sig) => sig,
                Err(err) => {
                    return Err(
                        PartialTicketbookVerificationFailure::MalformedBlindedSignature(err),
                    );
                }
            };

        let commitments =
            match IssuedTicketbooksDataResponseBody::try_get_private_attributes_commitments(
                partial_ticketbook,
            ) {
                Ok(cms) => cms,
                Err(err) => {
                    return Err(
                        PartialTicketbookVerificationFailure::MalformedPrivateAttributesCommitments(
                            err,
                        ),
                    );
                }
            };

        let public_attributes = [
            date_scalar(partial_ticketbook.expiration_date.ecash_unix_timestamp()),
            type_scalar(partial_ticketbook.ticketbook_type.encode()),
        ];

        #[allow(clippy::map_identity)]
        let attributes_refs = public_attributes.iter().collect::<Vec<_>>();

        // actually do verify the credential now
        if !verify_partial_blind_signature(
            &commitments,
            &attributes_refs,
            &blinded_sig,
            &self.details.verification_key,
        ) {
            return Err(PartialTicketbookVerificationFailure::InvalidSignature);
        }

        Ok(())
    }

    fn verify_challenge_response(&mut self, expiration_date: Date) {
        debug!("performing cryptographic verification on the data");

        // no point in continuing
        if self.caught_cheating() {
            return;
        }

        let Some(challenge_commitment) = &self.challenge_commitment_response else {
            return;
        };

        let merkle_proof = &challenge_commitment.body.merkle_proof;

        // aggregate all responses
        let mut all_ticketbook_data = BTreeMap::new();
        for res in &self.ticketbook_data_responses {
            all_ticketbook_data.extend(res.body.partial_ticketbooks.clone())
        }

        let num_ticketbooks = all_ticketbook_data.len();

        // 1. go through all requested partial ticketbooks and perform verification on them...
        for (deposit_id, partial_ticketbook) in all_ticketbook_data {
            // 1.1 does the deposit id match?
            if partial_ticketbook.deposit_id != deposit_id {
                error!("❗ TICKETBOOK DEPOSIT ID MISMATCH ❗");
                // the signatures will be in the evidence pack
                let evidence = self.produce_cheating_evidence(MismatchClaim {
                    actual: partial_ticketbook.deposit_id,
                    claimed: deposit_id,
                });
                self.set_banned_issuer("inconsistent partial ticketbook deposit id", evidence);
                return;
            }

            // 1.2 does the expiration date match?
            if partial_ticketbook.expiration_date != expiration_date {
                error!("❗ TICKETBOOK EXPIRATION DATE MISMATCH ❗");

                let evidence = self.produce_cheating_evidence(MismatchClaim {
                    actual: partial_ticketbook.expiration_date,
                    claimed: expiration_date,
                });
                self.set_banned_issuer("inconsistent partial ticketbook expiration date", evidence);
                return;
            }

            let recomputed_hash = partial_ticketbook.hash_to_merkle_leaf();

            // SAFETY: we already checked every deposit is included in the response
            #[allow(clippy::unwrap_used)]
            let expected_index = self.sampled_deposits.get(&deposit_id).unwrap().merkle_index;
            let expected_leaf = MerkleLeaf {
                hash: recomputed_hash.to_vec(),
                index: expected_index,
            };

            // 1.3 is this ticketbook actually included in the merkle proof?
            if !merkle_proof.contains_full_leaf(&expected_leaf) {
                error!("❗ MISSING MERKLE LEAF ❗");

                let evidence = self.produce_cheating_evidence(expected_leaf);
                self.set_banned_issuer("missing partial ticketbook merkle leaf", evidence);
                return;
            }

            // 1.4 is that partial ticketbook actually cryptographically valid?
            if let Err(verification_failure) = self.verify_partial_ticketbook(&partial_ticketbook) {
                error!("❗ PARTIAL TICKETBOOK VERIFICATION FAILURE ❗");

                let evidence = self.produce_cheating_evidence(GenericError {
                    error: verification_failure.to_string(),
                });
                self.set_banned_issuer("cryptographically malformed ticketbook", evidence);
                return;
            }
        }

        info!("✅ cryptographically verified all {num_ticketbooks} partial ticketbooks");
    }

    fn sample_deposits_for_challenge(&mut self, desired_amount: usize) {
        // no point in continuing
        if self.caught_cheating() {
            return;
        }

        if let Some(issued) = &self.issued_commitment {
            if desired_amount >= issued.body.deposits.len() {
                self.sampled_deposits = issued
                    .body
                    .deposits
                    .iter()
                    .cloned()
                    .map(|d| (d.deposit_id, d))
                    .collect();
            } else {
                let mut rng = rand::rng();
                self.sampled_deposits = issued
                    .body
                    .deposits
                    .sample(&mut rng, desired_amount)
                    .cloned()
                    .map(|d| (d.deposit_id, d))
                    .collect();
            }
        }
    }

    fn claimed_issued(&self) -> usize {
        match &self.issued_commitment {
            None => 0,
            Some(res) => res.body.deposits.len(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct VerificationConfig {
    /// Defines the minimum number of ticketbooks the monitor will validate
    /// regardless of the sampling rate
    pub min_validate_per_issuer: usize,

    /// The sampling rate of the issued ticketbooks
    pub sampling_rate: f64,

    /// Ratio of issuers that will undergo full verification as opposed to being let through.
    pub full_verification_ratio: f64,
}

pub struct TicketbookIssuanceVerifier<'a> {
    config: VerificationConfig,
    rewarder_keypair: &'a ed25519::KeyPair,

    whitelist: &'a [AccountId],
    banned_addresses: Vec<String>,
    expiration_date: Date,
    made_deposits: HashSet<DepositId>,
}

impl<'a> TicketbookIssuanceVerifier<'a> {
    pub fn new(
        config: VerificationConfig,
        rewarder_keypair: &'a ed25519::KeyPair,
        whitelist: &'a [AccountId],
        banned_addresses: Vec<String>,
        expiration_date: Date,
    ) -> Self {
        TicketbookIssuanceVerifier {
            config,
            rewarder_keypair,
            whitelist,
            banned_addresses,
            expiration_date,
            made_deposits: Default::default(),
        }
    }

    fn is_banned<C>(&self, issuer: &CredentialIssuer<C>) -> bool {
        self.banned_addresses
            .contains(&issuer.operator_account.to_string())
    }

    fn to_prebanned<C: NymApiClientExt>(&self, issuer: &CredentialIssuer<C>) -> OperatorIssuing {
        let whitelisted = self.whitelist.contains(&issuer.operator_account);

        OperatorIssuing {
            api_runner: issuer.api_client.api_url().to_string(),
            whitelisted,
            pre_banned: true,
            runner_account: issuer.operator_account.clone(),
            issued_ratio: Default::default(),
            skipped_verification: false,
            subsample_size: 0,
            issued_ticketbooks: 0,
            issuer_ban: None,
        }
    }

    fn to_result<C: NymApiClientExt + Sync>(&self, issuer: IssuerUnderTest<C>) -> OperatorIssuing {
        let whitelisted = self.whitelist.contains(&issuer.details.operator_account);
        let total_deposits = self.made_deposits.len();

        // an unaudited claim is divided by what others proved, so it is capped at a full slice
        let issued_ratio = if total_deposits == 0 {
            Decimal::zero()
        } else {
            Decimal::from_ratio(issuer.claimed_issued() as u32, total_deposits as u32)
                .min(Decimal::one())
        };

        OperatorIssuing {
            api_runner: issuer.details.api_client.api_url().to_string(),
            whitelisted,
            issued_ratio,
            issued_ticketbooks: issuer.claimed_issued() as u32,
            skipped_verification: issuer.verification_skipped,
            subsample_size: issuer.sampled_deposits.len() as u32,
            runner_account: issuer.details.operator_account,
            issuer_ban: issuer.issuer_ban,
            pre_banned: false,
        }
    }

    /// Adds what an audited, honest issuer demonstrably issued to the day's deposit union.
    fn record_made_deposits<C: NymApiClientExt + Sync>(&mut self, issuer: &IssuerUnderTest<C>) {
        if issuer.caught_cheating() || issuer.verification_skipped {
            return;
        }
        if let Some(commitment) = &issuer.issued_commitment {
            for deposit in &commitment.body.deposits {
                self.made_deposits.insert(deposit.deposit_id);
            }
        }
    }

    fn should_perform_full_verification(&self) -> bool {
        let mut rng = rand::rng();
        let choices = [true, false];
        let weights = [
            self.config.full_verification_ratio,
            1. - self.config.full_verification_ratio,
        ];

        #[allow(clippy::unwrap_used)]
        let verify_dist = WeightedIndex::new(weights).unwrap();
        let coin_toss_res = choices[verify_dist.sample(&mut rng)];
        debug!(
            "tossed a coin to see if the issuer should be fully verified, result: {coin_toss_res}"
        );
        coin_toss_res
    }

    fn desired_sample_size(&self, claimed_issued: usize) -> usize {
        max(
            self.config.min_validate_per_issuer,
            (claimed_issued as f64 * self.config.sampling_rate) as usize,
        )
    }

    #[instrument(
        skip_all,
        fields(
            ticketbook_expiration = %self.expiration_date,
        )
    )]
    pub async fn check_issuer<C: NymApiClientExt + Sync>(
        &mut self,
        issuer: CredentialIssuer<C>,
    ) -> Option<IssuerUnderTest<C>> {
        info!("beginning to check ticketbook issuance of {issuer}");

        let mut tested_issuer = IssuerUnderTest::new(issuer, *self.rewarder_keypair.public_key());

        // 1. attempt to get number of issued ticketbooks for given expiration date
        // the purpose of this query is two-fold: check if there's anything to challenge the issuer on
        // and see if it's running a recent enough version to support subsequent queries
        let issued_count = tested_issuer.get_issued_count(self.expiration_date).await;
        if issued_count == 0 {
            info!(
                "{} hasn't issued any ticketbooks with expiration on {} (or is running an outdated api). it will not undergo any further testing",
                tested_issuer.details, self.expiration_date
            );
            return None;
        }

        // 2. try to obtain commitments for issued ticketbooks (merkle root + deposit ids)
        // at this point if they refuse to give anything, we give them benefit of the doubt
        // and simply not reward them as opposed to banning them. they know we go around everyone
        // every day to get this information and there isn't any element of "chance" to be able to cheat on
        tested_issuer
            .get_issued_commitment(self.expiration_date)
            .await;

        if tested_issuer.caught_cheating() {
            return Some(tested_issuer);
        }

        // 3. toss a coin to see if we have to go through the full verification procedure
        if !self.should_perform_full_verification() {
            info!("ℹ️ full verification is getting skipped");
            tested_issuer.verification_skipped = true;
            return Some(tested_issuer);
        }

        // 4. sample deposits for the challenge (if applicable)
        // we want to sample at least the minimum specified amount or the desired ratio of all issued
        let desired_amount = self.desired_sample_size(tested_issuer.claimed_issued());
        tested_issuer.sample_deposits_for_challenge(desired_amount);

        // 5. issue the challenge to the issuer (if applicable) and get its commitment to the response
        // that includes the merkle proof to our sampled deposits
        tested_issuer
            .issue_deposit_challenge(self.rewarder_keypair.private_key(), self.expiration_date)
            .await;

        if tested_issuer.caught_cheating() {
            return Some(tested_issuer);
        }

        // 6. retrieve binary data of ticketbooks corresponding to the original challenge
        tested_issuer
            .get_ticketbooks_data(self.rewarder_keypair.private_key(), self.expiration_date)
            .await;

        if tested_issuer.caught_cheating() {
            return Some(tested_issuer);
        }

        // 7. verify the responses (if applicable)
        tested_issuer.verify_challenge_response(self.expiration_date);

        Some(tested_issuer)
    }

    #[instrument(
        skip_all,
        fields(
            ticketbook_expiration = %self.expiration_date,
        )
    )]
    pub async fn check_issuers<C: NymApiClientExt + Sync>(
        &mut self,
        issuers: Vec<CredentialIssuer<C>>,
    ) -> TicketbookIssuanceResults {
        info!("checking {} ticketbook issuers", issuers.len());

        let mut results = Vec::with_capacity(issuers.len());
        let mut tested = Vec::with_capacity(issuers.len());

        // we could parallelize it, but we're running the test so infrequently (relatively speaking)
        // that doing it sequentially is fine (probably...)
        for issuer in issuers {
            if self.is_banned(&issuer) {
                info!("not testing {issuer} as it's already been banned");
                results.push(self.to_prebanned(&issuer));
                continue;
            }

            if let Some(completed_test) = self.check_issuer(issuer).await {
                tested.push(completed_test);
            }
        }

        // every share is taken against the same, complete union, so it has to be built from
        // all the audits before any single result is computed
        for issuer in &tested {
            self.record_made_deposits(issuer);
        }
        results.extend(tested.into_iter().map(|issuer| self.to_result(issuer)));

        TicketbookIssuanceResults {
            approximate_deposits: self.made_deposits.len() as u32,
            api_runners: results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rewarder::ticketbook_issuance::test_harness::{FakeSigner, Misbehaviour};
    use time::macros::date;

    const COHORT: Date = date!(2026 - 09 - 20);

    fn audit_everyone() -> VerificationConfig {
        VerificationConfig {
            min_validate_per_issuer: 10,
            sampling_rate: 0.01,
            full_verification_ratio: 1.0,
        }
    }

    fn rewarder_keys() -> ed25519::KeyPair {
        nym_test_utils::helpers::dummy_ed25519_keypair(0)
    }

    async fn audit(signer: FakeSigner, config: VerificationConfig) -> IssuerUnderTest<FakeSigner> {
        let keys = rewarder_keys();
        let whitelist = vec![signer.operator_account()];
        let mut verifier =
            TicketbookIssuanceVerifier::new(config, &keys, &whitelist, vec![], COHORT);
        let issuer = signer.as_credential_issuer(1);
        verifier
            .check_issuer(issuer)
            .await
            .expect("an issuer that issued something is always tested")
    }

    fn ban_reason(tested: &IssuerUnderTest<FakeSigner>) -> Option<String> {
        tested.issuer_ban.as_ref().map(|b| b.reason.clone())
    }

    async fn audit_all(
        signers: Vec<FakeSigner>,
        config: VerificationConfig,
    ) -> Vec<OperatorIssuing> {
        let keys = rewarder_keys();
        let whitelist: Vec<AccountId> = signers.iter().map(|s| s.operator_account()).collect();
        let mut verifier =
            TicketbookIssuanceVerifier::new(config, &keys, &whitelist, vec![], COHORT);
        let mut issuers = Vec::new();
        for (i, signer) in signers.iter().enumerate() {
            issuers.push(signer.as_credential_issuer(i as u64 + 1));
        }
        verifier.check_issuers(issuers).await.api_runners
    }

    fn ratio_of(results: &[OperatorIssuing], account: &AccountId) -> Decimal {
        results
            .iter()
            .find(|r| &r.runner_account == account)
            .expect("issuer missing from results")
            .issued_ratio
    }

    #[tokio::test]
    async fn honest_issuer_with_full_sample_passes_audit() {
        let signer = FakeSigner::new(1, Misbehaviour::None);
        for deposit_id in 1..=5 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }

        let tested = audit(signer, audit_everyone()).await;

        assert_eq!(ban_reason(&tested), None);
        assert!(!tested.verification_skipped);
        assert_eq!(tested.claimed_issued(), 5);
        assert_eq!(tested.sampled_deposits.len(), 5);
        assert_eq!(tested.ticketbook_data_responses.len(), 1);
    }

    #[tokio::test]
    async fn honest_issuer_with_partial_sample_passes_audit() {
        let signer = FakeSigner::new(1, Misbehaviour::None);
        for deposit_id in 1..=100 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }

        let tested = audit(signer, audit_everyone()).await;

        assert_eq!(ban_reason(&tested), None);
        assert_eq!(tested.claimed_issued(), 100);
        assert_eq!(tested.sampled_deposits.len(), 10);
        assert_eq!(tested.ticketbook_data_responses.len(), 1);
    }

    #[tokio::test]
    async fn challenge_request_is_ordered_by_merkle_index() {
        let signer = FakeSigner::new(1, Misbehaviour::None);
        for deposit_id in 1..=100 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }

        let _ = audit(signer.clone(), audit_everyone()).await;

        // the rewarder must request the sampled deposits in merkle-index order: a signer that
        // builds the proof in request order (as nym-api does) otherwise produces a proof that
        // does not verify
        let indices = signer.last_challenge_indices();
        assert_eq!(indices.len(), 10);
        assert!(
            indices.windows(2).all(|w| w[0] < w[1]),
            "challenge leaf indices must be strictly ascending, got {indices:?}"
        );
    }

    #[tokio::test]
    async fn tampered_echo_of_the_challenge_request_is_banned() {
        let signer = FakeSigner::new(1, Misbehaviour::TamperedEcho);
        for deposit_id in 1..=5 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }

        let tested = audit(signer, audit_everyone()).await;

        assert_eq!(
            ban_reason(&tested).as_deref(),
            Some("original request body was tampered with")
        );
    }

    #[tokio::test]
    async fn short_data_batch_is_banned() {
        let signer = FakeSigner::new(1, Misbehaviour::ShortDataBatch);
        for deposit_id in 1..=5 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }

        let tested = audit(signer, audit_everyone()).await;

        assert_eq!(
            ban_reason(&tested).as_deref(),
            Some("incomplete response - requested 5 deposits but got 4 back")
        );
    }

    #[tokio::test]
    async fn ticketbook_signed_under_an_unadvertised_key_is_banned() {
        let signer = FakeSigner::new(1, Misbehaviour::None);
        for deposit_id in 1..=4 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }
        signer.issue_foreign_key_ticketbook(5, COHORT);

        let tested = audit(signer, audit_everyone()).await;

        assert_eq!(
            ban_reason(&tested).as_deref(),
            Some("cryptographically malformed ticketbook")
        );
    }

    #[tokio::test]
    async fn commitment_without_merkle_root_but_with_deposits_is_banned() {
        let signer = FakeSigner::new(1, Misbehaviour::NoMerkleRoot);
        for deposit_id in 1..=5 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }

        let tested = audit(signer, audit_everyone()).await;

        assert_eq!(
            ban_reason(&tested).as_deref(),
            Some("no merkle root for 2026-09-20 despite 5 committed deposits")
        );
        assert!(tested.challenge_commitment_response.is_none());
    }

    #[tokio::test]
    async fn commitment_without_merkle_root_is_banned_even_when_not_fully_audited() {
        let signer = FakeSigner::new(1, Misbehaviour::NoMerkleRoot);
        for deposit_id in 1..=5 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }

        let never_audit = VerificationConfig {
            full_verification_ratio: 0.0,
            ..audit_everyone()
        };
        let tested = audit(signer, never_audit).await;

        // the rootless commitment is a property of the commitment itself, so it is caught before
        // the sampling coin toss rather than skipped and paid on its claimed count
        assert_eq!(
            ban_reason(&tested).as_deref(),
            Some("no merkle root for 2026-09-20 despite 5 committed deposits")
        );
        assert!(!tested.verification_skipped);
    }

    #[tokio::test]
    async fn wrong_date_commitment_is_unrewarded_and_unpunished() {
        let signer = FakeSigner::new(1, Misbehaviour::WrongExpirationDate);
        for deposit_id in 1..=5 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }

        let tested = audit(signer, audit_everyone()).await;

        assert_eq!(ban_reason(&tested), None);
        assert!(tested.issued_commitment.is_none());
        assert_eq!(tested.claimed_issued(), 0);
        assert!(tested.sampled_deposits.is_empty());
    }

    #[tokio::test]
    async fn issued_ratio_does_not_depend_on_audit_order() {
        let full = FakeSigner::new(1, Misbehaviour::None);
        let lagging = FakeSigner::new(2, Misbehaviour::None);
        for deposit_id in 1..=100 {
            full.issue_ticketbook(deposit_id, COHORT);
        }
        for deposit_id in 1..=80 {
            lagging.issue_ticketbook(deposit_id, COHORT);
        }

        let expected_full = Decimal::one();
        let expected_lagging = Decimal::from_ratio(80u32, 100u32);

        let results = audit_all(vec![full.clone(), lagging.clone()], audit_everyone()).await;
        assert_eq!(ratio_of(&results, &full.operator_account()), expected_full);
        assert_eq!(
            ratio_of(&results, &lagging.operator_account()),
            expected_lagging
        );

        let results = audit_all(vec![lagging.clone(), full.clone()], audit_everyone()).await;
        assert_eq!(ratio_of(&results, &full.operator_account()), expected_full);
        assert_eq!(
            ratio_of(&results, &lagging.operator_account()),
            expected_lagging
        );
    }

    #[tokio::test]
    async fn unaudited_claim_never_exceeds_the_operator_slice() {
        let signer = FakeSigner::new(1, Misbehaviour::None);
        for deposit_id in 1..=100 {
            signer.issue_ticketbook(deposit_id, COHORT);
        }

        let keys = rewarder_keys();
        let whitelist = vec![signer.operator_account()];
        let never_audit = VerificationConfig {
            full_verification_ratio: 0.0,
            ..audit_everyone()
        };
        let mut verifier =
            TicketbookIssuanceVerifier::new(never_audit, &keys, &whitelist, vec![], COHORT);
        // what other, audited, issuers demonstrably issued
        verifier.made_deposits = (1..=80).collect();

        let issuer = signer.as_credential_issuer(1);
        let tested = verifier.check_issuer(issuer).await.unwrap();
        assert!(tested.verification_skipped);

        assert_eq!(verifier.to_result(tested).issued_ratio, Decimal::one());
    }
}
