// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::backend::fs_backend::error::StorageError;
use log::{error, info};
use sqlx::ConnectOptions;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct StorageManager {
    pub connection_pool: sqlx::SqlitePool,
}

// all SQL goes here
impl StorageManager {
    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
        // ensure the whole directory structure exists
        if let Some(parent_dir) = database_path.as_ref().parent() {
            std::fs::create_dir_all(parent_dir).map_err(|source| {
                StorageError::DatabasePathUnableToCreateParentDirectory {
                    provided_path: database_path.as_ref().to_path_buf(),
                    source,
                }
            })?;
        }

        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        opts.disable_statement_logging();

        let connection_pool = sqlx::SqlitePool::connect_with(opts)
            .await
            .map_err(|source| {
                error!("Failed to connect to SQLx database: {source}");
                StorageError::DatabaseConnectionError { source }
            })?;

        sqlx::migrate!("./fs_gateways_migrations")
            .run(&connection_pool)
            .await
            .inspect_err(|err| {
                error!("Failed to initialize SQLx database: {err}");
            })?;

        info!("Database migration finished!");
        Ok(StorageManager { connection_pool })
    }
}
