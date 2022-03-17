// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use credentials::error::Error as CredentialError;
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
}
