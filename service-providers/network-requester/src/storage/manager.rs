// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use sqlx::types::chrono::{DateTime, Utc};

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
    /// * `service_description`: Description of the service that gathered the data.
    /// * `request_processed_bytes`: Number of bytes for socks5 requests.
    /// * `response_processed_bytes`: Number of bytes for socks5 responses.
    /// * `interval_seconds`: Duration in seconds in which the data was gathered.
    pub(super) async fn insert_service_statistics(
        &self,
        service_description: String,
        request_processed_bytes: u32,
        response_processed_bytes: u32,
        interval_seconds: u32,
        timestamp: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO mixnet_statistics(service_description, request_processed_bytes, response_processed_bytes, interval_seconds, timestamp) VALUES (?, ?, ?, ?, ?)",
                service_description,
                request_processed_bytes,
                response_processed_bytes,
                interval_seconds,
                timestamp,
            ).execute(&self.connection_pool).await?;
        Ok(())
    }
}
