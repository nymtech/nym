// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymRewarderError;
use crate::rewarder::ticketbook_issuance::types::{
    CredentialIssuer, OperatorIssuing, TicketbookIssuanceResults,
};
use cosmwasm_std::Decimal;
use nym_compact_ecash::scheme::withdrawal::verify_partial_blind_signature;
use nym_compact_ecash::{date_scalar, type_scalar, CompactEcashError};
use nym_crypto::asymmetric::ed25519::{self, serde_helpers::bs58_ed25519_pubkey};
use nym_ecash_time::EcashTime;
use nym_ticketbooks_merkle::{IssuedTicketbook, MerkleLeaf};
use nym_validator_client::ecash::models::{
    CommitedDeposit, DepositId, IssuedTicketbooksChallengeResponse,
    IssuedTicketbooksChallengeResponseBody, IssuedTicketbooksForResponse,
};
use nym_validator_client::nyxd::AccountId;
use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use time::Date;
use tracing::{debug, info, info_span, warn};

const unused: &str = "add tracing like in monitor.rs";
const unused2: &str =
    "add intermediate logs to what's happening and results. like verifying merkle, etc.";

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
pub struct RegisteredPubKey {
    #[serde(with = "bs58_ed25519_pubkey")]
    registered_pub_key: ed25519::PublicKey,
}

#[derive(Serialize, Deserialize)]
pub struct MismatchResponse<T> {
    requested: T,
    received: T,
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
    commitment: Option<IssuedTicketbooksForResponse>,
    requested_challenge: Vec<DepositId>,
    challenge_response: Option<IssuedTicketbooksChallengeResponse>,

    #[serde(flatten)]
    inner: T,
}

pub struct IssuerUnderTest {
    details: CredentialIssuer,
    verification_skipped: bool,
    issuer_ban: Option<IssuerBan>,
    issued_commitment: Option<IssuedTicketbooksForResponse>,
    sampled_deposits: HashMap<DepositId, CommitedDeposit>,
    challenge_response: Option<IssuedTicketbooksChallengeResponse>,
}

impl IssuerUnderTest {
    fn new(details: CredentialIssuer) -> Self {
        IssuerUnderTest {
            details,
            verification_skipped: false,
            issuer_ban: None,
            issued_commitment: None,
            sampled_deposits: HashMap::new(),
            challenge_response: None,
        }
    }

    fn caught_cheating(&self) -> bool {
        self.issuer_ban.is_some()
    }

    fn produce_basic_cheating_evidence(&self) -> CheatingEvidence {
        self.produce_cheating_evidence(Empty {})
    }

    fn produce_cheating_evidence<T>(&self, additional_context: T) -> CheatingEvidence<T> {
        CheatingEvidence {
            commitment: self.issued_commitment.clone(),
            requested_challenge: self.sampled_deposits.keys().copied().collect(),
            challenge_response: self.challenge_response.clone(),
            inner: additional_context,
        }
    }

    // SAFETY: we're using stable serialisation
    #[allow(clippy::unwrap_used)]
    fn set_banned_issuer<T>(&mut self, reason: impl Into<String>, evidence: CheatingEvidence<T>)
    where
        T: Serialize,
    {
        self.issuer_ban = Some(IssuerBan {
            reason: reason.into(),
            serialised_evidence: serde_json::to_vec(&evidence).unwrap(),
        })
    }

    async fn get_issued_commitment(&mut self, expiration_date: Date) {
        debug!("getting issued ticketbooks information of {}", self.details);
        let issued_ticketbooks = match self
            .details
            .api_client
            .issued_ticketbooks_for(expiration_date)
            .await
        {
            Ok(res) => res,
            Err(err) => {
                info!("failed to obtain issued ticketbooks information from {}. it might be running an outdated api. the error was: {err}", self.details);
                return;
            }
        };

        // verify the signature on the response
        if !issued_ticketbooks.verify_signature(&self.details.public_key) {
            warn!(
                "issuer {} is cheating - failed to verify the signature on the issued response",
                self.details
            );
            let evidence = self.produce_cheating_evidence(RegisteredPubKey {
                registered_pub_key: self.details.public_key,
            });
            self.set_banned_issuer(
                format!("bad signature on the issued ticketbooks for {expiration_date}"),
                evidence,
            );
            return;
        }

        if expiration_date != issued_ticketbooks.body.expiration_date {
            warn!(
                "issuer {} might be cheating - received response for commitments with expiration {} while we requested {expiration_date}",
                self.details,
                issued_ticketbooks.body.expiration_date
            );
            let evidence = self.produce_cheating_evidence(MismatchResponse {
                requested: expiration_date,
                received: issued_ticketbooks.body.expiration_date,
            });
            self.set_banned_issuer(
                format!("bad ticketbook commitments for {expiration_date}"),
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

        let sampled = self.sampled_deposits.keys().copied().collect::<Vec<_>>();

        let challenge_response = match self
            .details
            .api_client
            .issued_ticketbooks_challenge(expiration_date, sampled.clone())
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
                    format!("no response for issued ticketbook challenge for {expiration_date}"),
                    evidence,
                );
                return;
            }
        };

        // verify the signature on the response
        if !challenge_response.verify_signature(&self.details.public_key) {
            warn!(
                "issuer {} is cheating - failed to verify the signature on the challenge response",
                self.details
            );
            let evidence = self.produce_cheating_evidence(RegisteredPubKey {
                registered_pub_key: self.details.public_key,
            });
            self.set_banned_issuer(
                format!("bad signature on the challenge response for {expiration_date}"),
                evidence,
            );
            return;
        }

        if expiration_date != challenge_response.body.expiration_date {
            warn!(
                "issuer {} is cheating - received response for challenge with expiration {} while we requested {expiration_date}",
                self.details,
                challenge_response.body.expiration_date
            );
            let evidence = self.produce_cheating_evidence(MismatchResponse {
                requested: expiration_date,
                received: challenge_response.body.expiration_date,
            });
            self.set_banned_issuer(
                format!("invalid deposits challenge response for {expiration_date}"),
                evidence,
            );
            return;
        }

        self.challenge_response = Some(challenge_response)
    }

    fn verify_partial_ticketbook(
        &self,
        partial_ticketbook: &IssuedTicketbook,
    ) -> Result<(), PartialTicketbookVerificationFailure> {
        let blinded_sig = match IssuedTicketbooksChallengeResponseBody::try_get_partial_credential(
            partial_ticketbook,
        ) {
            Ok(sig) => sig,
            Err(err) => {
                return Err(PartialTicketbookVerificationFailure::MalformedBlindedSignature(err))
            }
        };

        let commitments =
            match IssuedTicketbooksChallengeResponseBody::try_get_private_attributes_commitments(
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

        let Some(issued) = &self.issued_commitment else {
            return;
        };

        let Some(challenge) = &self.challenge_response else {
            return;
        };

        let issuer = &self.details;

        let partial_ticketbooks = &challenge.body.partial_ticketbooks;
        let merkle_proof = &challenge.body.merkle_proof;

        // 1. check if the response actually contains all the requested deposits
        for &deposit_id in self.sampled_deposits.keys() {
            if !partial_ticketbooks.contains_key(&deposit_id) {
                warn!("issuer {issuer} challenge response is missing deposit {deposit_id}!");
                let evidence = self.produce_basic_cheating_evidence();
                self.set_banned_issuer(
                    format!("requested deposit {deposit_id} is missing in challenge response"),
                    evidence,
                );
                return;
            }
        }

        // 2. check if the provided merkle proof has the same number of deposits as initially committed to
        if !merkle_proof.total_leaves() != issued.body.deposits.len() {
            warn!("issuer {issuer} merkle proof is for different number of deposits than initially committed to");
            let evidence = self.produce_cheating_evidence(MismatchClaim {
                actual: merkle_proof.total_leaves(),
                claimed: issued.body.deposits.len(),
            });
            self.set_banned_issuer("inconsistent number of merkle leaves", evidence);
            return;
        }

        // 3. attempt to extract the merkle root
        let merkle_root = match issued.body.merkle_root {
            None => {
                if !issued.body.deposits.is_empty() {
                    let evidence = self.produce_basic_cheating_evidence();
                    self.set_banned_issuer("unexpected empty merkle root", evidence);
                    return;
                }
                return;
            }
            Some(root) => root,
        };

        // 4. verify the actual proof
        if !merkle_proof.verify(merkle_root) {
            let evidence = self.produce_basic_cheating_evidence();
            self.set_banned_issuer("invalid merkle proof", evidence);
            return;
        }

        // 5. go through all requested partial ticketbooks and perform verification on them...
        for (&deposit_id, partial_ticketbook) in partial_ticketbooks {
            // 5.1 does the deposit id match?
            if partial_ticketbook.deposit_id != deposit_id {
                let evidence = self.produce_cheating_evidence(MismatchClaim {
                    actual: partial_ticketbook.deposit_id,
                    claimed: deposit_id,
                });
                self.set_banned_issuer("inconsistent partial ticketbook deposit id", evidence);
                return;
            }

            // 5.2 does the expiration date match?
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

            // 5.3 is this ticketbook actually included in the merkle proof?
            if !merkle_proof.contains_full_leaf(&expected_leaf) {
                let evidence = self.produce_cheating_evidence(expected_leaf);
                self.set_banned_issuer("missing partial ticketbook merkle leaf", evidence);
                return;
            }

            // 5.4 is that partial ticketbook actually cryptographically valid?
            if let Err(verification_failure) = self.verify_partial_ticketbook(partial_ticketbook) {
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

    whitelist: &'a [AccountId],
    banned_addresses: Vec<String>,
    expiration_date: Date,
    made_deposits: HashSet<DepositId>,
}

impl<'a> TicketbookIssuanceVerifier<'a> {
    pub fn new(
        config: VerificationConfig,
        whitelist: &'a [AccountId],
        banned_addresses: Vec<String>,
        expiration_date: Date,
    ) -> Self {
        TicketbookIssuanceVerifier {
            config,
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

        OperatorIssuing {
            api_runner: issuer.details.api_client.api_url().to_string(),
            whitelisted,
            issued_ratio: Decimal::from_ratio(
                issuer.claimed_issued() as u32,
                total_deposits as u32,
            ),
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

            let mut being_tested = IssuerUnderTest::new(issuer);

            // 1. try to obtain commitments for issued ticketbooks (merkle root + deposit ids)
            being_tested
                .get_issued_commitment(self.expiration_date)
                .await;

            issuers_being_tested.push(being_tested);
        }
        let a = info_span!("todo").entered();

        for issuer in issuers_being_tested.iter_mut() {
            // 2. toss a coin to see if we have to go through the full verification procedure
            if !self.should_perform_full_verification() {
                issuer.verification_skipped = true;
                continue;
            }

            // 3. sample deposits for the challenge (if applicable)
            // we want to sample at least the minimum specified amount or the desired ratio of all issued
            let desired_amount = max(
                self.config.min_validate_per_issuer,
                (issuer.claimed_issued() as f64 * self.config.sampling_rate) as usize,
            );
            issuer.sample_deposits_for_challenge(desired_amount);

            // 4. issue the challenge to the issuer (if applicable)
            issuer.issue_deposit_challenge(self.expiration_date).await;

            // 5. verify the response (if applicable)
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

        // 6. try to create summary of results produced
        for issuer in issuers_being_tested {
            results.push(self.to_result(issuer))
        }

        Ok(TicketbookIssuanceResults {
            approximate_deposits: self.made_deposits.len() as u32,
            api_runners: results,
        })
    }
}
