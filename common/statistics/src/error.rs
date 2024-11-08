// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Error types occurring while processing statistics events and reporting.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum StatsError {
    #[error("Failed to (de)serialize stats report : {0}")]
    ReportJsonSerialization(#[from] serde_json::Error),
}

/// Result of a statistics operation.
pub type Result<T> = core::result::Result<T, StatsError>;
