// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum StorageError {
    #[error("Database experienced an internal error: {0}")]
    InternalDatabase(#[from] sqlx::Error),

    #[error("Failed to perform database migration: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Somehow stored data is incorrect: {0}")]
    DataCorruption(String),
}
