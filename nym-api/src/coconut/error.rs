// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{response, Request, Response};
use std::io::Cursor;
use thiserror::Error;

use nym_crypto::asymmetric::{
    encryption::KeyRecoveryError,
    identity::{Ed25519RecoveryError, SignatureError},
};
use nym_dkg::error::DkgError;
use validator_client::nyxd::error::NyxdError;

use crate::node_status_api::models::NymApiStorageError;

pub type Result<T> = std::result::Result<T, CoconutError>;

#[derive(Debug, Error)]
pub enum CoconutError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Could not parse Ed25519 data - {0}")]
    Ed25519ParseError(#[from] Ed25519RecoveryError),

    #[error("Could not parse X25519 data - {0}")]
    X25519ParseError(#[from] KeyRecoveryError),

    #[error("Could not parse tx hash in request body")]
    TxHashParseError,

    #[error("Nyxd error - {0}")]
    NyxdError(#[from] NyxdError),

    #[error("Validator client error - {0}")]
    ValidatorClientError(#[from] validator_client::ValidatorClientError),

    #[error("Coconut internal error - {0}")]
    CoconutInternalError(#[from] nymcoconut::CoconutError),

    #[error("Could not find a deposit event in the transaction provided")]
    DepositEventNotFound,

    #[error("Could not find the deposit value in the event")]
    DepositValueNotFound,

    #[error("Could not find the deposit info in the event")]
    DepositInfoNotFound,

    #[error("Could not find the verification key in the event")]
    DepositVerifKeyNotFound,

    #[error("Could not find the encryption key in the event")]
    DepositEncrKeyNotFound,

    #[error("Signature didn't verify correctly")]
    SignatureVerificationError(#[from] SignatureError),

    #[error("Inconsistent public attributes")]
    InconsistentPublicAttributes,

    #[error(
        "Public attributes in request differ from the ones in deposit - Expected {0}, got {1}"
    )]
    DifferentPublicAttributes(String, String),

    #[error("Error in coconut interface - {0}")]
    CoconutInterfaceError(#[from] coconut_interface::error::CoconutInterfaceError),

    #[error("Storage error - {0}")]
    StorageError(#[from] NymApiStorageError),

    #[error("Credentials error - {0}")]
    CredentialsError(#[from] nym_credentials::error::Error),

    #[error("Incorrect credential proposal description: {reason}")]
    IncorrectProposal { reason: String },

    #[error("Invalid status of credential: {status}")]
    InvalidCredentialStatus { status: String },

    #[error("DKG error: {0}")]
    DkgError(#[from] DkgError),

    #[error("Failed to recover assigned node index: {reason}")]
    NodeIndexRecoveryError { reason: String },

    #[error("Unrecoverable state: {reason}. Process should be restarted")]
    UnrecoverableState { reason: String },

    #[error("DKG has not finished yet in order to derive the coconut key")]
    KeyPairNotDerivedYet,

    #[error("The coconut keypair is corrupted")]
    CorruptedCoconutKeyPair,

    #[error("There was a problem with the proposal id: {reason}")]
    ProposalIdError { reason: String },
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
