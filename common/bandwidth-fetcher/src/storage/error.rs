// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),

    #[error("migrate error")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("experienced internal storage error due to database inconsistency: {reason}")]
    DatabaseInconsistency { reason: String },

    #[error("failed to remove pending credential request storage")]
    RemoveStorage(#[from] std::io::Error),
}

impl StorageError {
    pub fn database_inconsistency(reason: impl Into<String>) -> StorageError {
        StorageError::DatabaseInconsistency {
            reason: reason.into(),
        }
    }
}
