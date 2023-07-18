// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTimeError;
use thiserror::Error;

use nym_credential_storage::error::StorageError;
use nym_credentials::error::Error as CredentialError;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::ValidatorClientError;

pub type Result<T> = std::result::Result<T, CredentialClientError>;

#[derive(Error, Debug)]
pub enum CredentialClientError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Bandwidth controller error: {0}")]
    BandwidthControllerError(#[from] nym_bandwidth_controller::error::BandwidthControllerError),

    #[error("Nyxd error: {0}")]
    Nyxd(#[from] NyxdError),

    #[error("Validator client error: {0}")]
    ValidatorClientError(#[from] ValidatorClientError),

    #[error("Credential error: {0}")]
    Credential(#[from] CredentialError),

    #[error("Could not use shared storage")]
    SharedStorageError(#[from] StorageError),

    #[error("Could not get system time")]
    SysTimeError(#[from] SystemTimeError),
}
