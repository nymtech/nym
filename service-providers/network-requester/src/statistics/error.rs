// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatsError {
    #[error("Bincode error: {0}")]
    BincodeError(#[from] bincode::Error),

    #[error("Reqwuest error {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Invalid stats provider client address")]
    InvalidClientAddress,
}
