// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_coconut_dkg_common::types::EpochId;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{response, Request, Response};
use std::io::Cursor;
use std::path::PathBuf;
use thiserror::Error;

use nym_crypto::asymmetric::{
    encryption::KeyRecoveryError,
    identity::{Ed25519RecoveryError, SignatureError},
};
use nym_dkg::error::DkgError;
use nym_pemstore::KeyPairPath;
use nym_validator_client::coconut::CoconutApiError;
use nym_validator_client::nyxd::error::{NyxdError, TendermintError};

use crate::node_status_api::models::NymApiStorageError;

pub type Result<T> = std::result::Result<T, CoconutError>;

#[derive(Debug, Error)]
pub enum CoconutError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error("coconut api query failure: {0}")]
    CoconutApiError(#[from] CoconutApiError),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("could not parse Ed25519 data: {0}")]
    Ed25519ParseError(#[from] Ed25519RecoveryError),

    #[error("could not parse X25519 data: {0}")]
    X25519ParseError(#[from] KeyRecoveryError),

    #[error("could not parse tx hash in request body: {source}")]
    TxHashParseError {
        #[source]
        source: TendermintError,
    },

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

    #[error("coconut internal error: {0}")]
    CoconutInternalError(#[from] nym_coconut::CoconutError),

    #[error("could not find a deposit event in the transaction provided")]
    DepositEventNotFound,

    #[error("could not find the deposit value in the event")]
    DepositValueNotFound,

    #[error("could not find the deposit info in the event")]
    DepositInfoNotFound,

    #[error("could not find the verification key in the event")]
    DepositVerifKeyNotFound,

    #[error("could not find the encryption key in the event")]
    DepositEncrKeyNotFound,

    #[error("signature didn't verify correctly")]
    SignatureVerificationError(#[from] SignatureError),

    #[error("inconsistent public attributes")]
    InconsistentPublicAttributes,

    #[error("the provided deposit value is inconsistent. got '{request}' while the value on chain is '{on_chain}'")]
    InconsistentDepositValue { request: String, on_chain: String },

    #[error("the provided deposit info is inconsistent. got '{request}' while the value on chain is '{on_chain}'")]
    InconsistentDepositInfo { request: String, on_chain: String },

    #[error("public attributes in request differ from the ones in deposit: Expected {0}, got {1}")]
    DifferentPublicAttributes(String, String),

    #[error("error in coconut interface: {0}")]
    CoconutInterfaceError(#[from] nym_coconut_interface::error::CoconutInterfaceError),

    #[error("storage error: {0}")]
    StorageError(#[from] NymApiStorageError),

    #[error("credentials error: {0}")]
    CredentialsError(#[from] nym_credentials::error::Error),

    #[error("incorrect credential proposal description: {reason}")]
    IncorrectProposal { reason: String },

    #[error("invalid status of credential: {status}")]
    InvalidCredentialStatus { status: String },

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

    #[error("failed to archive coconut key for epoch {epoch_id} using path {}: {source}", path.display())]
    KeyArchiveFailure {
        epoch_id: EpochId,
        path: PathBuf,

        // I hate that we're using anyhow error source here, but changing that would require bigger refactoring
        #[source]
        source: anyhow::Error,
    },

    #[error("there was a problem with the proposal id: {reason}")]
    ProposalIdError { reason: String },

    // I guess we should make this one a bit more detailed
    #[error("the provided query arguments were invalid")]
    InvalidQueryArguments,

    #[error("the internal dkg state for epoch {epoch_id} is missing - we might have joined mid exchange")]
    MissingDkgState { epoch_id: EpochId },

    #[error(
        "the node index value for epoch {epoch_id} is not available - are you sure we are a dealer?"
    )]
    UnavailableAssignedIndex { epoch_id: EpochId },

    #[error("the receiver index value for epoch {epoch_id} is not available - are you sure we are a receiver?")]
    UnavailableReceiverIndex { epoch_id: EpochId },

    #[error("the threshold value for epoch {epoch_id} is not available")]
    UnavailableThreshold { epoch_id: EpochId },

    #[error("insufficient number of dealings provided to derive the key")]
    InsufficientDealings {
        // TODO: details
    },
}

impl<'r, 'o: 'r> Responder<'r, 'o> for CoconutError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let err_msg = self.to_string();
        Response::build()
            .header(ContentType::Plain)
            .sized_body(err_msg.len(), Cursor::new(err_msg))
            .status(Status::BadRequest)
            .ok()
    }
}
