// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use sqlx::ConnectOptions;
use std::path::PathBuf;

use crate::statistics::StatsMessage;
use crate::storage::error::NetworkRequesterStorageError;
use crate::storage::manager::StorageManager;

mod error;
mod manager;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct NetworkRequesterStorage {
    manager: StorageManager,
}

impl NetworkRequesterStorage {
    pub async fn init(database_path: &PathBuf) -> Result<Self, NetworkRequesterStorageError> {
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        opts.disable_statement_logging();

        let connection_pool = sqlx::SqlitePool::connect_with(opts).await?;

        sqlx::migrate!("./migrations").run(&connection_pool).await?;
        info!("Database migration finished!");

        let storage = NetworkRequesterStorage {
            manager: StorageManager { connection_pool },
        };

        Ok(storage)
    }

    /// Adds an entry for some statistical data.
    ///
    /// # Arguments
    ///
    /// * `msg`: Message containing the statistical data.
    pub(super) async fn insert_service_statistics(
        &self,
        msg: StatsMessage,
    ) -> Result<(), sqlx::Error> {
        Ok(self
            .manager
            .insert_service_statistics(
                msg.description,
                msg.request_data.total_processed_bytes(),
                msg.response_data.total_processed_bytes(),
            )
            .await?)
    }
}
