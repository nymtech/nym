// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use credential_storage::error::StorageError;
use credentials::error::Error as CredentialError;
use crypto::asymmetric::encryption::KeyRecoveryError;
use crypto::asymmetric::identity::Ed25519RecoveryError;
use validator_client::nymd::error::NymdError;
use validator_client::ValidatorClientError;

pub type Result<T> = std::result::Result<T, CredentialClientError>;

#[derive(Error, Debug)]
pub enum CredentialClientError {
    #[error("Nymd error: {0}")]
    Nymd(#[from] NymdError),

    #[error("Validator client error: {0}")]
    ValidatorClientError(#[from] ValidatorClientError),

    #[error("Credential error: {0}")]
    Credential(#[from] CredentialError),

    #[error("The tx hash provided is not valid")]
    InvalidTxHash,

    #[error("Could not parse Ed25519 data")]
    Ed25519ParseError(#[from] Ed25519RecoveryError),

    #[error("Could not parse X25519 data")]
    X25519ParseError(#[from] KeyRecoveryError),

    #[error("Could not use shared storage")]
    SharedStorageError(#[from] StorageError),
}
