// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use validator_client::nymd::error::NymdError;

pub type Result<T> = std::result::Result<T, CredentialClientError>;

#[derive(Error, Debug)]
pub enum CredentialClientError {
    #[error("Nymd error: {0}")]
    Nymd(#[from] NymdError),
}
