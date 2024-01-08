// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

pub type Result<T> = std::result::Result<T, EphemeraError>;

#[derive(Debug, Error)]
pub enum EphemeraError {
    #[error("Validator client error - {0}")]
    ValidatorClientError(#[from] nym_validator_client::ValidatorClientError),

    #[error("Nyxd error - {0}")]
    NyxdError(#[from] nym_validator_client::nyxd::error::NyxdError),
}
