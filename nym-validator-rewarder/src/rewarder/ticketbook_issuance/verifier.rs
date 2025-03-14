// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymRewarderError;
use crate::rewarder::ticketbook_issuance::types::{
    CredentialIssuer, OperatorIssuing, TicketbookIssuanceResults,
};
use cosmwasm_std::Decimal;
use nym_compact_ecash::scheme::withdrawal::verify_partial_blind_signature;
use nym_compact_ecash::{date_scalar, type_scalar, CompactEcashError};
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::EcashTime;
use nym_network_defaults::MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE;
use nym_ticketbooks_merkle::{IssuedTicketbook, MerkleLeaf};
use nym_validator_client::ecash::models::{
    CommitedDeposit, DepositId, IssuedTicketbooksChallengeCommitmentResponse,
    IssuedTicketbooksDataRequestBody, IssuedTicketbooksDataResponse,
    IssuedTicketbooksDataResponseBody, IssuedTicketbooksForResponse, SignableMessageBody,
    SignedMessage,
};
use nym_validator_client::nyxd::AccountId;
use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::collections::{BTreeMap, HashMap, HashSet};
use thiserror::Error;
use time::Date;
use tracing::{debug, info, instrument, warn};

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
pub struct SignedMismatchClaim<T, R> {
    claimed: T,
    actual: T,
    signed_response: R,
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

pub struct IssuerUnderTest {
    rewarder_pubkey: ed25519::PublicKey,
    details: CredentialIssuer,
    verification_skipped: bool,
    issuer_ban: Option<IssuerBan>,
    issued_commitment: Option<IssuedTicketbooksForResponse>,
    sampled_deposits: HashMap<DepositId, CommitedDeposit>,
    challenge_commitment_response: Option<IssuedTicketbooksChallengeCommitmentResponse>,
    ticketbook_data_responses: Vec<IssuedTicketbooksDataResponse>,
}

impl IssuerUnderTest {
    fn new(details: CredentialIssuer, rewarder_pubkey: ed25519::PublicKey) -> Self {
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
                let evidence = self.produce_basic_cheating_evidence();
                self.set_banned_issuer(
                    format!("bad signature on the data response for {expiration_date} that included deposits {batch:?} "),
                    evidence,
                );
                return;
            }

            // 2. check if the signature on original request still matches
            if self.ban_if_tampered_request(&data_response) {
                return;
            }

            // 3. make sure every requested deposit is in the response
            if batch.len() != data_response.body.partial_ticketbooks.len() {
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
                    let evidence = self.produce_cheating_evidence(data_response);
                    self.set_banned_issuer(
                        format!("incomplete response - {deposit_id} is missing"),
                        evidence,
                    );
                    return;
                }
            }

            // 4. append results to the total
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
                debug!(
                    "{} claims to have issued {} ticketbooks with expiration on {expiration_date}",
                    self.details, expiration_date
                );
                res.total
            }
            Err(err) => {
                warn!(
                    "{} does not support queries required for determining issuance rewards: {err}",
                    self.details
                );
                0
            }
        }
    }

    async fn get_issued_commitment(&mut self, expiration_date: Date) {
        debug!(
            "getting issued ticketbooks information of {}...",
            self.details
        );
        let issued_ticketbooks = match self
            .details
            .api_client
            .issued_ticketbooks_for(expiration_date)
            .await
        {
            Ok(res) => res,
            Err(err) => {
                warn!("failed to obtain issued ticketbooks information from {}. it might be running an outdated api. the error was: {err}", self.details);
                return;
            }
        };

        // 1. check if the signature on the response matches
        if !issued_ticketbooks.verify_signature(&self.details.public_key) {
            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!("bad signature on the issued ticketbooks for {expiration_date}"),
                evidence,
            );
            return;
        }

        // 2. check if the signature on original request still matches
        if self.ban_if_tampered_request(&issued_ticketbooks) {
            return;
        }

        if expiration_date != issued_ticketbooks.body.expiration_date {
            // we know our request wasn't tampered with, so the issuer simply returned data for wrong date
            let evidence = self.produce_cheating_evidence(MismatchResponse {
                requested: expiration_date,
                received: issued_ticketbooks.body.expiration_date,
                signed_response: issued_ticketbooks,
            });
            self.set_banned_issuer(
                format!("bad ticketbooks data for {expiration_date}"),
                evidence,
            );
            return;
        }

        self.issued_commitment = Some(issued_ticketbooks)
    }

    async fn issue_deposit_challenge(&mut self, expiration_date: Date) {
        // no point in continuing
        if self.caught_cheating() {
            return;
        }

        // nothing to challenge on
        if self.sampled_deposits.is_empty() {
            return;
        }

        // if the root is empty, it means there were no issued ticketbooks
        let Some(merkle_root) = self.issued_merkle_root_commitment() else {
            return;
        };

        // if they claimed they haven't issued anything - no point in making any challenges

        let sampled = self.sampled_deposits.keys().copied().collect::<Vec<_>>();

        // 1. get the response
        let challenge_commitment = match self
            .details
            .api_client
            .issued_ticketbooks_challenge_commitment(expiration_date, sampled.clone())
            .await
        {
            Ok(res) => res,
            Err(err) => {
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

        // 2. check if the signature on the response matches
        if !challenge_commitment.verify_signature(&self.details.public_key) {
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
            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!(
                    "max data request size below minimum of {MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE}"
                ),
                evidence,
            );
        }

        // 4. check if the signature on original request still matches
        if !self.ban_if_tampered_request(&challenge_commitment) {
            return;
        }

        // 5. verify whether the expiration date matches the requested value
        if expiration_date != challenge_commitment.body.expiration_date {
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
            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!("invalid merkle proof for {expiration_date}"),
                evidence,
            );
            return;
        }

        // 6.2. check if the provided merkle proof has the same number of deposits as initially committed to
        if merkle_proof.total_leaves() != sampled.len() {
            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer(
                format!("invalid merkle proof for {expiration_date} - {} leaves present whilst {} deposits got sampled", merkle_proof.total_leaves(), sampled.len()),
                evidence,
            );
            return;
        }

        self.challenge_commitment_response = Some(challenge_commitment)
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
                    )
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
                    )
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

        // 1. go through all requested partial ticketbooks and perform verification on them...
        for (deposit_id, partial_ticketbook) in all_ticketbook_data {
            // 1.1 does the deposit id match?
            if partial_ticketbook.deposit_id != deposit_id {
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
                let evidence = self.produce_cheating_evidence(expected_leaf);
                self.set_banned_issuer("missing partial ticketbook merkle leaf", evidence);
                return;
            }

            // 1.4 is that partial ticketbook actually cryptographically valid?
            if let Err(verification_failure) = self.verify_partial_ticketbook(&partial_ticketbook) {
                let evidence = self.produce_cheating_evidence(GenericError {
                    error: verification_failure.to_string(),
                });
                self.set_banned_issuer("cryptographically malformed ticketbook", evidence);
                return;
            }
        }
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
                let mut rng = thread_rng();
                self.sampled_deposits = issued
                    .body
                    .deposits
                    .choose_multiple(&mut rng, desired_amount)
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

    fn is_banned(&self, issuer: &CredentialIssuer) -> bool {
        self.banned_addresses
            .contains(&issuer.operator_account.to_string())
    }

    fn to_prebanned(&self, issuer: &CredentialIssuer) -> OperatorIssuing {
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

    fn to_result(&self, issuer: IssuerUnderTest) -> OperatorIssuing {
        let whitelisted = self.whitelist.contains(&issuer.details.operator_account);
        let total_deposits = self.made_deposits.len();

        let issued_ratio = if total_deposits == 0 {
            Decimal::zero()
        } else {
            Decimal::from_ratio(issuer.claimed_issued() as u32, total_deposits as u32)
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

    fn should_perform_full_verification(&self) -> bool {
        let mut rng = thread_rng();
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

    #[instrument(
        skip_all,
        fields(
            ticketbook_expiration = %self.expiration_date,
        )
    )]
    pub async fn check_issuers(
        &mut self,
        issuers: Vec<CredentialIssuer>,
    ) -> Result<TicketbookIssuanceResults, NymRewarderError> {
        info!("checking {} ticketbook issuers", issuers.len());

        let mut issuers_being_tested = Vec::with_capacity(issuers.len());
        let mut results = Vec::with_capacity(issuers.len());

        // we could parallelize it, but we're running the test so infrequently (relatively speaking)
        // that doing it sequentially is fine (probably...)
        for issuer in issuers {
            if self.is_banned(&issuer) {
                info!("not testing {issuer} as it's already been banned");
                results.push(self.to_prebanned(&issuer));
                continue;
            }

            let mut being_tested =
                IssuerUnderTest::new(issuer, *self.rewarder_keypair.public_key());

            // 1. attempt to get number of issued ticketbooks for given expiration date
            // the purpose of this query is two-fold: check if there's anything to challenge the issuer on
            // and see if it's running a recent enough version to support subsequent queries
            let issued_count = being_tested.get_issued_count(self.expiration_date).await;
            if issued_count == 0 {
                continue;
            }

            // 2. try to obtain commitments for issued ticketbooks (merkle root + deposit ids)
            being_tested
                .get_issued_commitment(self.expiration_date)
                .await;

            issuers_being_tested.push(being_tested);
        }

        for issuer in issuers_being_tested.iter_mut() {
            // 3. toss a coin to see if we have to go through the full verification procedure
            if !self.should_perform_full_verification() {
                issuer.verification_skipped = true;
                continue;
            }

            // 4. sample deposits for the challenge (if applicable)
            // we want to sample at least the minimum specified amount or the desired ratio of all issued
            let desired_amount = max(
                self.config.min_validate_per_issuer,
                (issuer.claimed_issued() as f64 * self.config.sampling_rate) as usize,
            );
            issuer.sample_deposits_for_challenge(desired_amount);

            // 5. issue the challenge to the issuer (if applicable) and get its commitment to the response
            // that includes the merkle proof to our sampled deposits
            issuer.issue_deposit_challenge(self.expiration_date).await;

            // 6. retrieve binary data of ticketbooks corresponding to the original challenge
            issuer
                .get_ticketbooks_data(self.rewarder_keypair.private_key(), self.expiration_date)
                .await;

            // 7. verify the responses (if applicable)
            issuer.verify_challenge_response(self.expiration_date);

            // if issuer produced valid results, try to update global deposit ids
            if !issuer.caught_cheating() && issuer.claimed_issued() > 0 {
                if let Some(commitment) = &issuer.issued_commitment {
                    for deposit in &commitment.body.deposits {
                        self.made_deposits.insert(deposit.deposit_id);
                    }
                }
            }
        }

        // 7. try to create summary of results produced
        for issuer in issuers_being_tested {
            results.push(self.to_result(issuer))
        }

        Ok(TicketbookIssuanceResults {
            approximate_deposits: self.made_deposits.len() as u32,
            api_runners: results,
        })
    }
}
