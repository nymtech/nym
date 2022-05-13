// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::ConnectOptions;
use std::path::PathBuf;

use crate::statistics::StatsMessage;
use crate::storage::error::NetworkRequesterStorageError;
use crate::storage::manager::StorageManager;
use crate::storage::models::MixnetStatistics;
pub(crate) use crate::storage::routes::post_mixnet_statistics;

mod error;
mod manager;
mod models;
mod routes;

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
    ) -> Result<(), NetworkRequesterStorageError> {
        let timestamp: DateTime<Utc> = DateTime::parse_from_rfc3339(&msg.timestamp)
            .map_err(|_| NetworkRequesterStorageError::TimestampParse)?
            .into();
        Ok(self
            .manager
            .insert_service_statistics(
                msg.description,
                msg.request_data.total_processed_bytes(),
                msg.response_data.total_processed_bytes(),
                msg.interval_seconds,
                timestamp,
            )
            .await?)
    }

    /// Returns data submitted within the provided time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: indicates the lower bound timestamp for the data, RFC 3339 format
    /// * `until`: indicates the upper bound timestamp for the data, RFC 3339 format
    pub(super) async fn get_service_statistics_in_interval(
        &self,
        since: &str,
        until: &str,
    ) -> Result<Vec<MixnetStatistics>, NetworkRequesterStorageError> {
        let since = DateTime::parse_from_rfc3339(since)
            .map_err(|_| NetworkRequesterStorageError::TimestampParse)?
            .into();
        let until = DateTime::parse_from_rfc3339(until)
            .map_err(|_| NetworkRequesterStorageError::TimestampParse)?
            .into();
        Ok(self
            .manager
            .get_service_statistics_in_interval(since, until)
            .await?)
    }
}
