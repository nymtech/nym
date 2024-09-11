// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credential_storage::error::StorageError;
use nym_credentials::error::Error as CredentialsError;
use nym_credentials_interface::CompactEcashError;
use nym_crypto::asymmetric::encryption::KeyRecoveryError;
use nym_crypto::asymmetric::identity::Ed25519RecoveryError;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::error::ValidatorClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BandwidthControllerError {
    #[error("Nyxd error: {0}")]
    Nyxd(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("coconut api query failure: {0}")]
    CoconutApiError(#[from] EcashApiError),

    #[error("There was a credential storage error - {0}")]
    CredentialStorageError(Box<dyn std::error::Error + Send + Sync>),

    #[error("the credential storage does not contain any usable credentials")]
    NoCredentialsAvailable,

    // this should really be fully incorporated into the above, but messing with coconut is the last thing I want to do now
    #[error(transparent)]
    StorageError(#[from] StorageError),

    #[error("Ecash error - {0}")]
    EcashError(#[from] CompactEcashError),

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

    #[error("can't handle recovering storage with revision {stored}. {expected} was expected")]
    UnsupportedCredentialStorageRevision { stored: u8, expected: u8 },

    #[error("did not receive a valid response for aggregated data ({typ}) from ANY nym-api")]
    ExhaustedApiQueries { typ: String },
}

impl BandwidthControllerError {
    pub fn credential_storage_error(
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        BandwidthControllerError::CredentialStorageError(Box::new(source))
    }
}
