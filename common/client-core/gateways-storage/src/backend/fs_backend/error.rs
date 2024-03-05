// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::BadGateway;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("the provided database path doesn't have a filename defined")]
    DatabasePathWithoutFilename { provided_path: PathBuf },

    #[error("unable to create the directory for the database at {}: {source}", provided_path.display())]
    DatabasePathUnableToCreateParentDirectory {
        provided_path: PathBuf,
        source: io::Error,
    },

    #[error("failed to perform sqlx migration: {source}")]
    MigrationError {
        #[source]
        #[from]
        source: sqlx::migrate::MigrateError,
    },

    #[error("failed to connect to the underlying connection pool: {source}")]
    DatabaseConnectionError {
        #[source]
        source: sqlx::error::Error,
    },

    #[error("failed to run the SQL query: {source}")]
    QueryError {
        #[source]
        #[from]
        source: sqlx::error::Error,
    },

    #[error(transparent)]
    MalformedGateway(#[from] BadGateway),
}
