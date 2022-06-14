// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::ConnectOptions;
use std::path::PathBuf;

use statistics_common::StatsMessage;

use crate::storage::error::NetworkStatisticsStorageError;
use crate::storage::manager::StorageManager;
use crate::storage::models::ServiceStatistics;

pub(crate) mod error;
mod manager;
mod models;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct NetworkStatisticsStorage {
    manager: StorageManager,
}

impl NetworkStatisticsStorage {
    pub async fn init(base_dir: &PathBuf) -> Result<Self, NetworkStatisticsStorageError> {
        std::fs::create_dir_all(base_dir)?;
        let database_path = base_dir.join("db.sqlite");
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        opts.disable_statement_logging();

        let connection_pool = sqlx::SqlitePool::connect_with(opts).await?;

        sqlx::migrate!("./migrations").run(&connection_pool).await?;
        info!("Database migration finished!");

        let storage = NetworkStatisticsStorage {
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
    ) -> Result<(), NetworkStatisticsStorageError> {
        let timestamp: DateTime<Utc> = DateTime::parse_from_rfc3339(&msg.timestamp)
            .map_err(|_| NetworkStatisticsStorageError::TimestampParse)?
            .into();
        for stats_data in msg.stats_data {
            match stats_data {
                statistics_common::StatsData::Service(service_data) => {
                    self.manager
                        .insert_service_statistics(
                            service_data.requested_service.clone(),
                            service_data.request_bytes,
                            service_data.response_bytes,
                            msg.interval_seconds,
                            timestamp,
                        )
                        .await?;
                }
                statistics_common::StatsData::Gateway(_) => todo!(),
            }
        }

        Ok(())
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
    ) -> Result<Vec<ServiceStatistics>, NetworkStatisticsStorageError> {
        let since = DateTime::parse_from_rfc3339(since)
            .map_err(|_| NetworkStatisticsStorageError::TimestampParse)?
            .into();
        let until = DateTime::parse_from_rfc3339(until)
            .map_err(|_| NetworkStatisticsStorageError::TimestampParse)?
            .into();
        Ok(self
            .manager
            .get_service_statistics_in_interval(since, until)
            .await?)
    }
}
