// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use credential_storage::error::StorageError;
use credentials::error::Error as CredentialError;
use crypto::asymmetric::encryption::KeyRecoveryError;
use crypto::asymmetric::identity::Ed25519RecoveryError;
use validator_client::nymd::error::NymdError;

pub type Result<T> = std::result::Result<T, CredentialClientError>;

#[derive(Error, Debug)]
pub enum CredentialClientError {
    #[error("Nymd error: {0}")]
    Nymd(#[from] NymdError),

    #[error("Credential error: {0}")]
    Credential(#[from] CredentialError),

    #[error("No previous deposit with that tx hash")]
    NoDeposit,

    #[error("Wrong number of attributes")]
    WrongAttributeNumber,

    #[error("Could not find any backed up blind sign request data")]
    NoLocalBlindSignRequest,

    #[error("The local blind sign request data is corrupted")]
    CorruptedBlindSignRequest,

    #[error("The tx hash provided is not valid")]
    InvalidTxHash,

    #[error("Could not parse Ed25519 data")]
    Ed25519ParseError(#[from] Ed25519RecoveryError),

    #[error("Could not parse X25519 data")]
    X25519ParseError(#[from] KeyRecoveryError),

    #[error("Could not use shared storage")]
    SharedStorageError(#[from] StorageError),
}
