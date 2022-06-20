// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatsError {
    #[error("Reqwuest error {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Invalid stats provider client address")]
    InvalidClientAddress,

    #[error("Common statistics error {0}")]
    CommonError(#[from] statistics_common::error::StatsError),
}
