// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatsError {
    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
}
