// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Database experienced an internal error - {0}")]
    InternalDatabaseError(#[from] sqlx::Error),

    #[error("experienced internal storage error due to database inconsistency: {reason}")]
    DatabaseInconsistency { reason: String },

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Failed to perform database migration - {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),

    #[error("Inconsistent data in database")]
    InconsistentData,

    #[error("No unused credential in database. You need to buy at least one")]
    NoCredential,

    #[error("No signatures for epoch {epoch_id} in the database")]
    NoSignatures { epoch_id: i64 },

    #[error("Database unique constraint violation. Is the credential already imported?")]
    ConstraintUnique,
}

impl StorageError {
    pub fn database_inconsistency<S: Into<String>>(reason: S) -> StorageError {
        StorageError::DatabaseInconsistency {
            reason: reason.into(),
        }
    }
}
