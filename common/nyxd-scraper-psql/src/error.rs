// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nyxd_scraper_shared::helpers::MalformedDataError;
use nyxd_scraper_shared::storage::NyxdScraperStorageError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PostgresScraperError {
    #[error("experienced internal database error: {0}")]
    InternalDatabaseError(#[from] sqlx::error::Error),

    #[error("failed to perform startup SQL migration: {0}")]
    StartupMigrationFailure(#[from] sqlx::migrate::MigrateError),

    #[error("failed to begin storage tx: {source}")]
    StorageTxBeginFailure {
        #[source]
        source: sqlx::error::Error,
    },

    #[error("failed to commit storage tx: {source}")]
    StorageTxCommitFailure {
        #[source]
        source: sqlx::error::Error,
    },

    #[error(transparent)]
    MalformedData(#[from] MalformedDataError),

    // TOOD: add struct name
    #[error("json serialisation failure: {source}")]
    SerialisationFailure {
        #[from]
        source: serde_json::Error,
    },
}

impl From<PostgresScraperError> for NyxdScraperStorageError {
    fn from(err: PostgresScraperError) -> Self {
        NyxdScraperStorageError::new(err)
    }
}
