// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StatsStorageError {
    #[error("Database experienced an internal error: {0}")]
    InternalDatabaseError(#[from] sqlx::Error),

    #[error("Failed to perform database migration: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
}
