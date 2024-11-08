// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatsError {
    #[error("Failed to (de)serialize stats report : {0}")]
    ReportJsonSerialization(#[from] serde_json::Error),

    #[error("Failed to deserialize stats report : {0}")]
    ReportBytesDeserialization(String),
}
