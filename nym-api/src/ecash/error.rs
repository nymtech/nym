// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::NymApiStorageError;
use nym_coconut_dkg_common::types::{ChunkIndex, DealingIndex, EpochId};
use nym_credentials_interface::UnknownTicketType;
use nym_crypto::asymmetric::{
    encryption::KeyRecoveryError,
    identity::{Ed25519RecoveryError, SignatureError},
};
use nym_dkg::error::DkgError;
use nym_dkg::Threshold;
use nym_ecash_contract_common::deposit::DepositId;
use nym_ecash_contract_common::redeem_credential::BATCH_REDEMPTION_PROPOSAL_TITLE;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::AccountId;
use okapi::openapi3::Responses;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{response, Request, Response};
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::util::ensure_status_code_exists;
use std::io::Cursor;
use std::num::ParseIntError;
use thiserror::Error;
use time::error::ComponentRange;
use time::OffsetDateTime;

pub type Result<T, E = EcashError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum EcashError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error("the address of the bandwidth contract hasn't been set")]
    MissingBandwidthContractAddress,

    #[error("the current bandwidth contract does not have any admin address set")]
    MissingBandwidthContractAdmin,

    #[error("failed to derive the admin account from the provided public key: {formatted_source}")]
    AdminAccountDerivationFailure { formatted_source: String },

    #[error("only secp256k1 keys are supported for free pass issuance")]
    UnsupportedNonSecp256k1Key,

    #[error("failed to parse the free pass expiry date: {source}")]
    ExpiryDateParsingFailure {
        #[source]
        source: ParseIntError,
    },

    #[error("the provided expiration date is too late")]
    ExpirationDateTooLate,

    #[error("the provided expiration date is too early")]
    ExpirationDateTooEarly,

    #[error("the provided expiration date is malformed")]
    MalformedExpirationDate { raw: String },

    #[error("failed to parse expiry timestamp into proper datetime: {source}")]
    InvalidExpiryDate {
        unix_timestamp: i64,
        #[source]
        source: ComponentRange,
    },

    #[error("the received bandwidth voucher did not contain deposit value")]
    MissingBandwidthValue,

    #[error("failed to parse the bandwidth voucher value: {source}")]
    VoucherValueParsingFailure {
        #[source]
        source: ParseIntError,
    },

    #[error("coconut api query failure: {0}")]
    CoconutApiError(#[from] EcashApiError),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("could not parse Ed25519 data: {0}")]
    Ed25519ParseError(#[from] Ed25519RecoveryError),

    #[error("could not parse X25519 data: {0}")]
    X25519ParseError(#[from] KeyRecoveryError),

    #[error("could not get transaction details for '{tx_hash}': {source}")]
    TxRetrievalFailure {
        tx_hash: String,
        #[source]
        source: NyxdError,
    },

    #[error("nyxd error: {0}")]
    NyxdError(#[from] NyxdError),

    #[error("validator client error: {0}")]
    ValidatorClientError(#[from] nym_validator_client::ValidatorClientError),

    #[error("Compact ecash internal error - {0}")]
    CompactEcashInternalError(#[from] nym_compact_ecash::error::CompactEcashError),

    #[error("Account linked to this public key has been blacklisted")]
    BlacklistedAccount,

    #[error("could not find a deposit event in the transaction provided")]
    DepositEventNotFound,

    #[error("could not find the deposit info in the event")]
    DepositInfoNotFound,

    #[error("signature didn't verify correctly")]
    SignatureVerificationError(#[from] SignatureError),

    #[error("storage error: {0}")]
    StorageError(#[from] NymApiStorageError),

    #[error("credentials error: {0}")]
    CredentialsError(#[from] nym_credentials::error::Error),

    #[error("incorrect credential proposal description: {reason}")]
    IncorrectProposal { reason: String },

    #[error("DKG error: {0}")]
    DkgError(#[from] DkgError),

    #[error("failed to recover assigned node index: {reason}")]
    NodeIndexRecoveryError { reason: String },

    #[error("unrecoverable state: {reason}")]
    UnrecoverableState { reason: String },

    #[error("DKG has not finished yet in order to derive the coconut key")]
    KeyPairNotDerivedYet,

    #[error("the coconut keypair is corrupted")]
    CorruptedCoconutKeyPair,

    #[error("there was a problem with the proposal id: {reason}")]
    ProposalIdError { reason: String },

    // I guess we should make this one a bit more detailed
    #[error("the provided query arguments were invalid")]
    InvalidQueryArguments,

    #[error("the internal dkg state for epoch {epoch_id} is missing - we might have joined mid exchange")]
    MissingDkgState { epoch_id: EpochId },

    #[error("a new iteration of DKG is currently in progress. all ticket issuance is halted until that's completed")]
    DkgInProgress,

    #[error(
        "the node index value for epoch {epoch_id} is not available - are you sure we are a dealer?"
    )]
    UnavailableAssignedIndex { epoch_id: EpochId },

    #[error("the receiver index value for epoch {epoch_id} is not available - are you sure we are a receiver?")]
    UnavailableReceiverIndex { epoch_id: EpochId },

    #[error("the threshold value for epoch {epoch_id} is not available")]
    UnavailableThreshold { epoch_id: EpochId },

    #[error("the proposal id value for epoch {epoch_id} is not available")]
    UnavailableProposalId { epoch_id: EpochId },

    #[error("could not find dealing chunk {chunk_index} for dealing {dealing_index} from dealer {dealer} for epoch {epoch_id} on the chain!")]
    MissingDealingChunk {
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    },

    #[error("could not find ecash deposit associated with id {deposit_id}")]
    NonExistentDeposit { deposit_id: DepositId },

    #[error("the provided request digest does not match the hash of attached serial numbers")]
    MismatchedRequestDigest,

    #[error("the on chain proposal digest does not match the attached request digest")]
    MismatchedOnChainDigest,

    #[error("one of the attached tickets {serial_number_bs58} has not been verified before")]
    TicketNotVerified { serial_number_bs58: String },

    #[error("the provided ticket(s) redemption proposal is invalid: {source}")]
    RedemptionProposalFailure {
        #[from]
        source: RedemptionError,
    },

    #[error("this gateway hasn't submitted any tickets for verification")]
    NotTicketsProvided,

    #[error("this gateway is attempting to redeem its tickets too often. last redemption happened on {last_redemption}. the earliest next permitted redemption will be on {next_allowed}")]
    TooFrequentRedemption {
        last_redemption: OffsetDateTime,
        next_allowed: OffsetDateTime,
    },

    #[error(
        "could not sign the data for epoch {requested}. our current key is for epoch {available}"
    )]
    InvalidSigningKeyEpoch {
        requested: EpochId,
        available: EpochId,
    },

    #[error("could not obtain enough shares for aggregation. got {shares} shares whilst the threshold is {threshold}")]
    InsufficientNumberOfShares { threshold: Threshold, shares: usize },

    #[error(transparent)]
    UnknownTicketBookType(#[from] UnknownTicketType),
}

impl<'r, 'o: 'r> Responder<'r, 'o> for EcashError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let err_msg = self.to_string();
        Response::build()
            .header(ContentType::Plain)
            .sized_body(err_msg.len(), Cursor::new(err_msg))
            .status(Status::BadRequest)
            .ok()
    }
}

impl OpenApiResponderInner for EcashError {
    fn responses(_gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        let mut responses = Responses::default();
        ensure_status_code_exists(&mut responses, 400);
        Ok(responses)
    }
}

#[derive(Debug, Error)]
pub enum RedemptionError {
    #[error("failed to retrieve proposal {proposal_id} from the chain")]
    ProposalRetrievalFailure { proposal_id: u64 },

    #[error(
        "the proposal {proposal_id} has invalid title. got {received} but expected {}",
        BATCH_REDEMPTION_PROPOSAL_TITLE
    )]
    InvalidProposalTitle { proposal_id: u64, received: String },

    #[error("the proposal {proposal_id} has invalid description. got {received} but expected {expected}")]
    InvalidProposalDescription {
        proposal_id: u64,
        received: String,
        expected: String,
    },

    #[error("the proposal {proposal_id} is still pending")]
    StillPending { proposal_id: u64 },

    #[error("the proposal {proposal_id} has already been executed")]
    AlreadyExecuted { proposal_id: u64 },

    #[error("the proposal {proposal_id} has already been rejected")]
    AlreadyRejected { proposal_id: u64 },

    #[error("the proposal {proposal_id} has already been passed")]
    AlreadyPassed { proposal_id: u64 },

    #[error("the proposal {proposal_id} was proposed by an unexpected address {received}. expected the ecash contract at {expected}")]
    InvalidProposer {
        proposal_id: u64,
        received: String,
        expected: AccountId,
    },

    #[error(
        "the proposal {proposal_id} did not contain exactly a single contract execution message"
    )]
    TooManyMessages { proposal_id: u64 },

    #[error("the proposal {proposal_id} did not contain the correct redemption execution message")]
    InvalidMessage { proposal_id: u64 },

    #[error("the proposal {proposal_id} has not been made against the expected e-cash contract")]
    InvalidContract { proposal_id: u64 },

    #[error("the proposal {proposal_id} proposes redemption of tickets for gateway {proposed}, but the request has been sent by {received}")]
    InvalidRedemptionTarget {
        proposal_id: u64,
        proposed: String,
        received: String,
    },

    #[error("the proposal {proposal_id} proposes redemption of {proposed} tickets, but the request has been sent for {received} instead")]
    InvalidRedemptionTicketCount {
        proposal_id: u64,
        proposed: u16,
        received: u16,
    },
}
