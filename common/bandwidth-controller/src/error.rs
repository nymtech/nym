// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_coconut_interface::CoconutError;
use nym_credential_storage::error::StorageError;
use nym_credentials::error::Error as CredentialsError;
use nym_validator_client::error::ValidatorClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BandwidthControllerError {
    #[error("There was a credential storage error - {0}")]
    CredentialStorageError(#[from] StorageError),

    #[error("Coconut error - {0}")]
    CoconutError(#[from] CoconutError),

    #[error("Validator client error - {0}")]
    ValidatorError(#[from] ValidatorClientError),

    #[error("Credential error - {0}")]
    CredentialError(#[from] CredentialsError),
}
