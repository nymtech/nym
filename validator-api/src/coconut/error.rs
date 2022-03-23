// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{response, Request, Response};
use std::io::Cursor;
use thiserror::Error;

use crypto::asymmetric::{
    encryption::KeyRecoveryError,
    identity::{Ed25519RecoveryError, SignatureError},
};
use validator_client::nymd::error::NymdError;

pub type Result<T> = std::result::Result<T, CoconutError>;

#[derive(Debug, Error)]
pub enum CoconutError {
    #[error("Could not parse Ed25519 data")]
    Ed25519ParseError(#[from] Ed25519RecoveryError),

    #[error("Could not parse X25519 data")]
    X25519ParseError(#[from] KeyRecoveryError),

    #[error("Could not parse tx hash in request body")]
    TxHashParseError,

    #[error("Nymd error - {0}")]
    NymdError(#[from] NymdError),

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

    #[error("Sled error - {0}")]
    SledError(#[from] sled::Error),

    #[error("No signature found")]
    NoSignature,
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
