// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::models::PersistedBandwidth;
use time::OffsetDateTime;

#[derive(Clone)]
pub(crate) struct BandwidthManager {
    connection_pool: sqlx::SqlitePool,
}

impl BandwidthManager {
    /// Creates new instance of the `BandwidthManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        BandwidthManager { connection_pool }
    }

    /// Creates a new bandwidth entry for the particular client.
    pub(crate) async fn insert_new_client(&self, client_id: i64) -> Result<(), sqlx::Error> {
        // FIXME: hack; we need to change api slightly
        sqlx::query!(
            "INSERT INTO available_bandwidth(client_id, available, expiration) VALUES (?, 0, ?)",
            client_id,
            OffsetDateTime::UNIX_EPOCH,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Set the expiration date of the particular client to the provided date.
    pub(crate) async fn set_expiration(
        &self,
        client_id: i64,
        expiration: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET expiration = ?
                WHERE client_id = ?
            "#,
            expiration,
            client_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Reset all the bandwidth associated with the freepass and reset its expiration date
    pub(crate) async fn reset_bandwidth(&self, client_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET available = 0, expiration = ?
                WHERE client_id = ?
            "#,
            OffsetDateTime::UNIX_EPOCH,
            client_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Tries to retrieve available bandwidth for the particular client.
    pub(crate) async fn get_available_bandwidth(
        &self,
        client_id: i64,
    ) -> Result<Option<PersistedBandwidth>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM available_bandwidth WHERE client_id = ?")
            .bind(client_id)
            .fetch_optional(&self.connection_pool)
            .await
    }

    pub(crate) async fn increase_bandwidth(
        &self,
        client_id: i64,
        amount: i64,
    ) -> Result<i64, sqlx::Error> {
        let mut tx = self.connection_pool.begin().await?;
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET available = available + ?
                WHERE client_id = ?
            "#,
            amount,
            client_id
        )
        .execute(&mut tx)
        .await?;

        let remaining = sqlx::query!(
            "SELECT available FROM available_bandwidth WHERE client_id = ?",
            client_id
        )
        .fetch_one(&mut tx)
        .await?
        .available;

        tx.commit().await?;
        Ok(remaining)
    }

    pub(crate) async fn revoke_ticket_bandwidth(
        &self,
        ticket_id: i64,
        amount: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET available = available - ?
                WHERE client_id = (SELECT client_id FROM received_ticket WHERE id = ?)
            "#,
            amount,
            ticket_id,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn decrease_bandwidth(
        &self,
        client_id: i64,
        amount: i64,
    ) -> Result<i64, sqlx::Error> {
        let mut tx = self.connection_pool.begin().await?;
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET available = available - ?
                WHERE client_id = ?
            "#,
            amount,
            client_id
        )
        .execute(&mut tx)
        .await?;

        let remaining = sqlx::query!(
            "SELECT available FROM available_bandwidth WHERE client_id = ?",
            client_id
        )
        .fetch_one(&mut tx)
        .await?
        .available;

        tx.commit().await?;
        Ok(remaining)
    }
}
