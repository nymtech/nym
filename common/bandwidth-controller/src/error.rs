// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_coconut_interface::CoconutError;
use nym_credential_storage::error::StorageError;
use nym_credentials::error::Error as CredentialsError;
use nym_crypto::asymmetric::encryption::KeyRecoveryError;
use nym_crypto::asymmetric::identity::Ed25519RecoveryError;
use nym_validator_client::error::ValidatorClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BandwidthControllerError {
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Nyxd error: {0}")]
    Nyxd(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("There was a credential storage error - {0}")]
    CredentialStorageError(#[from] StorageError),

    #[error("Coconut error - {0}")]
    CoconutError(#[from] CoconutError),

    #[error("Validator client error - {0}")]
    ValidatorError(#[from] ValidatorClientError),

    #[error("Credential error - {0}")]
    CredentialError(#[from] CredentialsError),

    #[error("Could not parse Ed25519 data")]
    Ed25519ParseError(#[from] Ed25519RecoveryError),

    #[error("Could not parse X25519 data")]
    X25519ParseError(#[from] KeyRecoveryError),

    #[error("The tx hash provided is not valid")]
    InvalidTxHash,

    #[error("Threshold not set yet")]
    NoThreshold,
}
