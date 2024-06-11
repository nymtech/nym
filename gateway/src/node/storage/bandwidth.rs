// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::models::{PersistedBandwidth, SpentCredential};
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
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client.
    pub(crate) async fn insert_new_client(
        &self,
        client_address_bs58: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO available_bandwidth(client_address_bs58, available, expiration) VALUES (?, 0, ?)",
            client_address_bs58,
            OffsetDateTime::UNIX_EPOCH,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Set the expiration date of the particular client to the provided date.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client.
    /// * `expiration`: the expiration date
    pub(crate) async fn set_expiration(
        &self,
        client_address_bs58: &str,
        expiration: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET expiration = ?
                WHERE client_address_bs58 = ?
            "#,
            expiration,
            client_address_bs58
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Reset all the bandwidth associated with the freepass and reset its expiration date
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client.
    pub(crate) async fn reset_bandwidth(
        &self,
        client_address_bs58: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET available = 0, expiration = ?
                WHERE client_address_bs58 = ?
            "#,
            OffsetDateTime::UNIX_EPOCH,
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
        sqlx::query_as("SELECT * FROM available_bandwidth WHERE client_address_bs58 = ?")
            .bind(client_address_bs58)
            .fetch_optional(&self.connection_pool)
            .await
    }

    /// Sets available bandwidth of the particular client to the provided amount;
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    /// * `amount`: the updated client bandwidth amount.
    pub(crate) async fn set_available_bandwidth(
        &self,
        client_address_bs58: &str,
        amount: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE available_bandwidth
                SET available = ?
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
    /// * `client_address_bs58`: address of the client that spent the credential
    pub(crate) async fn insert_spent_credential(
        &self,
        blinded_serial_number_bs58: &str,
        client_address_bs58: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO spent_credential
                (blinded_serial_number_bs58, client_address_bs58)
                VALUES (?, ?)
            "#,
            blinded_serial_number_bs58,
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
