// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::storage::models::PendingStoredCredential;

#[derive(Clone)]
pub(crate) struct CredentialManager {
    connection_pool: sqlx::SqlitePool,
}

impl CredentialManager {
    /// Creates new instance of the `CredentialManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        CredentialManager { connection_pool }
    }

    /// Inserts provided credential into the database.
    /// If the credential previously existed for the provided client, they are overwritten with the new data.
    ///
    /// # Arguments
    ///
    /// * `credential`: base58 representation of an ecash credential
    pub(crate) async fn insert_credential(&self, credential: String) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO credentials(credentials) VALUES (?)",
            credential
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Store a pending credential
    ///
    /// # Arguments
    ///
    /// * `pending`: pending credential to store
    pub(crate) async fn insert_pending_credential(
        &self,
        credential: String,
        gateway_address: String,
        api_url: String,
        proposal_id: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT OR REPLACE INTO pending(credential, gateway_address, api_urls, proposal_id) VALUES (?, ?, ?, ?)",
            credential,
            gateway_address,
            api_url,
            proposal_id,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Remove a pending credential
    ///
    /// # Arguments
    ///
    /// * `id`: id of the pending credential to remove
    pub(crate) async fn remove_pending_credential(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM pending WHERE id = ?", id)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    /// Get all pending credentials
    ///
    pub(crate) async fn get_all_pending_credential(
        &self,
    ) -> Result<Vec<PendingStoredCredential>, sqlx::Error> {
        let res = sqlx::query_as!(PendingStoredCredential, "SELECT * FROM pending")
            .fetch_all(&self.connection_pool)
            .await?;
        Ok(res)
    }
}
