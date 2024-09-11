// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::PersistedSharedKeys;

#[derive(Clone)]
pub(crate) struct SharedKeysManager {
    connection_pool: sqlx::SqlitePool,
}

impl SharedKeysManager {
    /// Creates new instance of the `SharedKeysManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        SharedKeysManager { connection_pool }
    }

    pub(crate) async fn client_id(&self, client_address_bs58: &str) -> Result<i64, sqlx::Error> {
        let client_id = sqlx::query!(
            "SELECT id FROM shared_keys WHERE client_address_bs58 = ?",
            client_address_bs58
        )
        .fetch_one(&self.connection_pool)
        .await?
        .id;
        Ok(client_id)
    }

    /// Inserts provided derived shared keys into the database.
    /// If keys previously existed for the provided client, they are overwritten with the new data.
    ///
    /// # Arguments
    ///
    /// * `shared_keys`: shared encryption (AES128CTR) and mac (hmac-blake3) derived shared keys to store.
    pub(crate) async fn insert_shared_keys(
        &self,
        client_address_bs58: String,
        derived_aes128_ctr_blake3_hmac_keys_bs58: String,
    ) -> Result<i64, sqlx::Error> {
        // https://stackoverflow.com/a/20310838
        // we don't want to be using `INSERT OR REPLACE INTO` due to the foreign key on `available_bandwidth` if the entry already exists
        sqlx::query!(
            r#"
                INSERT OR IGNORE INTO shared_keys(client_address_bs58, derived_aes128_ctr_blake3_hmac_keys_bs58) VALUES (?, ?);
                UPDATE shared_keys SET derived_aes128_ctr_blake3_hmac_keys_bs58 = ? WHERE client_address_bs58 = ?
            "#,
            client_address_bs58,
            derived_aes128_ctr_blake3_hmac_keys_bs58,
            derived_aes128_ctr_blake3_hmac_keys_bs58,
            client_address_bs58,
        ).execute(&self.connection_pool).await?;

        self.client_id(&client_address_bs58).await
    }

    /// Tries to retrieve shared keys stored for the particular client.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client
    pub(crate) async fn get_shared_keys(
        &self,
        client_address_bs58: &str,
    ) -> Result<Option<PersistedSharedKeys>, sqlx::Error> {
        sqlx::query_as!(
            PersistedSharedKeys,
            "SELECT * FROM shared_keys WHERE client_address_bs58 = ?",
            client_address_bs58
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Removes from the database shared keys derived with the particular client.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client
    // currently there is no code flow that causes removal (not overwriting)
    // of the stored keys. However, retain the function for consistency and completion sake
    #[allow(dead_code)]
    pub(crate) async fn remove_shared_keys(
        &self,
        client_address_bs58: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM shared_keys WHERE client_address_bs58 = ?",
            client_address_bs58
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
