// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use sqlx::types::chrono::{DateTime, Utc};

use crate::storage::models::MixnetStatistics;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

// all SQL goes here
impl StorageManager {
    /// Adds an entry for some statistical data.
    ///
    /// # Arguments
    ///
    /// * `requested_service`: Address of the service requested.
    /// * `request_processed_bytes`: Number of bytes for socks5 requests.
    /// * `response_processed_bytes`: Number of bytes for socks5 responses.
    /// * `interval_seconds`: Duration in seconds in which the data was gathered.
    pub(super) async fn insert_service_statistics(
        &self,
        requested_service: String,
        request_processed_bytes: u32,
        response_processed_bytes: u32,
        interval_seconds: u32,
        timestamp: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO mixnet_statistics(requested_service, request_processed_bytes, response_processed_bytes, interval_seconds, timestamp) VALUES (?, ?, ?, ?, ?)",
            requested_service,
            request_processed_bytes,
            response_processed_bytes,
            interval_seconds,
            timestamp,
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    /// Returns data submitted within the provided time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: indicates the lower bound timestamp for the data
    /// * `until`: indicates the upper bound timestamp for the data
    pub(super) async fn get_service_statistics_in_interval(
        &self,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<Vec<MixnetStatistics>, sqlx::Error> {
        sqlx::query_as!(
            MixnetStatistics,
            "SELECT * FROM mixnet_statistics WHERE timestamp BETWEEN ? AND ?",
            since,
            until
        )
        .fetch_all(&self.connection_pool)
        .await
    }
}
