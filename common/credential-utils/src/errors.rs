// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credential_storage::error::StorageError;
use nym_credentials::error::Error as CredentialError;
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
    Nyxd(#[from] NyxdError),

    #[error(transparent)]
    Credential(#[from] CredentialError),

    #[error("Could not use shared storage: {0}")]
    SharedStorageError(#[from] StorageError),

    #[error("failed to parse credential value: {0}")]
    MalformedCredentialValue(#[from] ParseIntError),
}
