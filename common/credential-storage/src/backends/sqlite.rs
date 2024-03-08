// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::CoinIndicesSignature;
use crate::models::StoredIssuedCredential;

#[derive(Clone)]
pub struct CoconutCredentialManager {
    connection_pool: sqlx::SqlitePool,
}

impl CoconutCredentialManager {
    /// Creates new instance of the `CoconutCredentialManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub fn new(connection_pool: sqlx::SqlitePool) -> Self {
        CoconutCredentialManager { connection_pool }
    }

    pub async fn insert_issued_credential(
        &self,
        credential_type: String,
        serialization_revision: u8,
        credential_data: &[u8],
        epoch_id: u32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO coconut_credentials(serialization_revision, credential_type, credential_data, epoch_id, consumed, expired)
                VALUES (?, ?, ?, ?, false, false)
            "#,
            serialization_revision, credential_type, credential_data, epoch_id
        ).execute(&self.connection_pool).await?;
        Ok(())
    }

    pub async fn get_next_unspent_credential(
        &self,
    ) -> Result<Option<StoredIssuedCredential>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM coconut_credentials WHERE NOT consumed AND NOT expired LIMIT 1",
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Consumes in the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `id`: Database id.
    pub async fn consume_coconut_credential(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE coconut_credentials SET consumed = TRUE WHERE id = ?",
            id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Marks the specified credential as expired
    ///
    /// # Arguments
    ///
    /// * `id`: Id of the credential to mark as expired.
    pub async fn mark_expired(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE coconut_credentials SET expired = TRUE WHERE id = ?",
            id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Inserts provided coin_indices_signatures into the database.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    /// * `coin_indices_signatures` : The coin indices signatures for the epoch
    pub async fn insert_coin_indices_sig(
        &self,
        epoch_id: String,
        coin_indices_sig: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO coin_indices_signatures(epoch_id, signatures) VALUES (?, ?)",
            epoch_id,
            coin_indices_sig
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Check if coin indices signatures are present for a given epoch
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    pub async fn is_coin_indices_sig_present(&self, epoch_id: String) -> Result<bool, sqlx::Error> {
        sqlx::query!(
            "SELECT epoch_id FROM coin_indices_signatures WHERE epoch_id = ?",
            epoch_id
        )
        .fetch_optional(&self.connection_pool)
        .await
        .map(|r| r.is_some())
    }

    /// Get coin_indices_signatures of a given epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    pub async fn get_coin_indices_sig(
        &self,
        epoch_id: String,
    ) -> Result<Option<CoinIndicesSignature>, sqlx::Error> {
        sqlx::query_as!(
            CoinIndicesSignature,
            "SELECT * FROM coin_indices_signatures WHERE epoch_id = ?",
            epoch_id
        )
        .fetch_optional(&self.connection_pool)
        .await
    }
}
