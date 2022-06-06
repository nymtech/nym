// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum NetworkStatisticsStorageError {
    #[error("File system error - {0}")]
    FSError(#[from] std::io::Error),

    #[error("SQL error - {0}")]
    InternalDatabaseError(#[from] sqlx::Error),

    #[error("SQL migrate error - {0}")]
    DatabaseMigrateError(#[from] sqlx::migrate::MigrateError),

    #[error("Timestamp could not be parsed")]
    TimestampParse,
}
