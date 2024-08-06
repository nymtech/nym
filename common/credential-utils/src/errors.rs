// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credential_storage::error::StorageError;
use nym_credentials::error::Error as CredentialError;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nyxd::error::NyxdError;
use std::num::ParseIntError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    BandwidthControllerError(#[from] nym_bandwidth_controller::error::BandwidthControllerError),

    #[error(transparent)]
    EcashApiError(#[from] EcashApiError),

    #[error(transparent)]
    Nyxd(#[from] NyxdError),

    #[error(transparent)]
    Credential(#[from] CredentialError),

    #[error("could not use shared storage: {0}")]
    SharedStorageError(Box<dyn std::error::Error + Send + Sync>),

    #[error("failed to parse credential value: {0}")]
    MalformedCredentialValue(#[from] ParseIntError),
}

impl Error {
    pub fn storage_error(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Error::SharedStorageError(Box::new(source))
    }
}

impl From<StorageError> for Error {
    fn from(value: StorageError) -> Self {
        Self::storage_error(value)
    }
}
