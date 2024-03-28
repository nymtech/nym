// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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

    #[error("failed to rename our databse file - {source}")]
    DatabaseRenameError {
        #[source]
        source: io::Error,
    },

    #[error("failed to rename our old databse file - {source}")]
    DatabaseOldFileRemoveError {
        #[source]
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

    #[error("The loaded data is inconsistent - it seems that on the last shutdown the client hasn't finished the data flush. You may have to remove the entire storage manually")]
    IncompleteDataFlush,

    #[error("data retrieved from the underlying storage is corrupted: {details}")]
    CorruptedData {
        details: String,
        // err: Option<Box<dyn std::error::Error>>
    },

    #[error("failed to create storage")]
    FailedToCreateStorage {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
