// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::models::{PersistedBandwidth, SpentCredential};

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

    /// Mark received credential as spent and insert it into the storage.
    ///
    /// # Arguments
    ///
    /// * `blinded_serial_number_bs58`: the unique blinded serial number embedded in the credential
    /// * `was_freepass`: indicates whether the spent credential was a freepass
    /// * `client_address_bs58`: address of the client that spent the credential
    pub(crate) async fn insert_spent_credential(
        &self,
        blinded_serial_number_bs58: &str,
        was_freepass: bool,
        client_address_bs58: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO spent_credential
                (blinded_serial_number_bs58, was_freepass, client_address_bs58)
                VALUES (?, ?, ?)
            "#,
            blinded_serial_number_bs58,
            was_freepass,
            client_address_bs58
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Retrieve the spent credential with the provided blinded serial number from the storage.
    ///
    /// # Arguments
    ///
    /// * `blinded_serial_number_bs58`: the unique blinded serial number embedded in the credential
    pub(crate) async fn retrieve_spent_credential(
        &self,
        blinded_serial_number_bs58: &str,
    ) -> Result<Option<SpentCredential>, sqlx::Error> {
        sqlx::query_as!(
            SpentCredential,
            r#"
                SELECT * FROM spent_credential
                WHERE blinded_serial_number_bs58 = ?
                LIMIT 1
            "#,
            blinded_serial_number_bs58,
        )
        .fetch_optional(&self.connection_pool)
        .await
    }
}
