// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::error::ClientCoreError;
use nym_id::NymIdError;

#[derive(thiserror::Error, Debug)]
pub enum StatsCollectorError {
    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    // TODO: add more details here
    #[error("failed to validate the loaded config")]
    ConfigValidationFailure,

    #[error("failed to connect to mixnet: {source}")]
    FailedToConnectToMixnet { source: nym_sdk::Error },

    #[error("failed to load configuration file: {0}")]
    FailedToLoadConfig(String),

    #[error("failed to setup mixnet client: {source}")]
    FailedToSetupMixnetClient { source: nym_sdk::Error },

    #[error("Stats error : {0}")]
    StatsError(#[from] nym_statistics_common::error::StatsError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    NymIdError(#[from] NymIdError),

    #[error("Storage error : {0}")]
    ReportStorageError(#[from] crate::storage::error::ClientStatsReportStorageError),
}

pub type Result<T> = std::result::Result<T, StatsCollectorError>;
