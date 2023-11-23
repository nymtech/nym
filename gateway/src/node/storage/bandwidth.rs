// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::models::PersistedBandwidth;

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
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client.
    pub(crate) async fn insert_new_client(
        &self,
        client_address_bs58: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO available_bandwidth(client_address_bs58, available) VALUES (?, 0)",
            client_address_bs58
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Tries to retrieve available bandwidth for the particular client.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client.
    pub(crate) async fn get_available_bandwidth(
        &self,
        client_address_bs58: &str,
    ) -> Result<Option<PersistedBandwidth>, sqlx::Error> {
        sqlx::query_as!(
            PersistedBandwidth,
            "SELECT * FROM available_bandwidth WHERE client_address_bs58 = ?",
            client_address_bs58
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Increases available bandwidth of the particular client by the specified amount.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client.
    /// * `amount`: amount of available bandwidth to be added to the client.
    pub(crate) async fn increase_available_bandwidth(
        &self,
        client_address_bs58: &str,
        amount: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET available = available + ?
                WHERE client_address_bs58 = ?
            "#,
            amount,
            client_address_bs58
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Decreases available bandwidth of the particular client by the specified amount.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client.
    /// * `amount`: amount of available bandwidth to be removed from the client.
    pub(crate) async fn decrease_available_bandwidth(
        &self,
        client_address_bs58: &str,
        amount: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET available = available - ?
                WHERE client_address_bs58 = ?
            "#,
            amount,
            client_address_bs58
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
