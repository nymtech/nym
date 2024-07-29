// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::storage::models::{EpochCredentials, IssuedCredential};
use crate::support::storage::manager::StorageManager;
use nym_coconut_dkg_common::types::EpochId;

#[async_trait]
pub trait CoconutStorageManagerExt {
    /// Gets the information about all issued partial credentials in this (coconut) epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the (coconut) epoch in question.
    async fn get_epoch_credentials(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochCredentials>, sqlx::Error>;

    /// Creates new entry for EpochCredentials for this (coconut) epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the (coconut) epoch in question.
    #[allow(dead_code)]
    async fn create_epoch_credentials_entry(&self, epoch_id: EpochId) -> Result<(), sqlx::Error>;

    /// Update the EpochCredentials by incrementing the total number of issued credentials,
    /// and setting `start_id` if unset (i.e. this is the first credential issued this epoch)
    ///
    /// # Arguments
    /// * `epoch_id`: Id of the (coconut) epoch in question.
    /// * `credential_id`: (database) Id of the coconut credential that triggered the update.
    async fn update_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
        credential_id: i64,
    ) -> Result<(), sqlx::Error>;

    /// Attempts to retrieve an issued credential from the data store.    
    ///
    /// # Arguments
    ///
    /// * `credential_id`: (database) id of the issued credential
    async fn get_issued_credential(
        &self,
        credential_id: i64,
    ) -> Result<Option<IssuedCredential>, sqlx::Error>;

    /// Attempts to retrieve an issued credential from the data store.
    ///
    /// # Arguments
    ///
    /// * `tx_hash`: transaction hash of the deposit used in the issued bandwidth credential
    async fn get_issued_bandwidth_credential_by_hash(
        &self,
        tx_hash: &str,
    ) -> Result<Option<IssuedCredential>, sqlx::Error>;

    /// Store the provided issued credential information and return its (database) id.
    ///
    /// # Arguments
    ///
    /// * `credential`: partial credential, alongside any data required for verification.
    async fn store_issued_credential(
        &self,
        epoch_id: u32,
        tx_hash: String,
        bs58_partial_credential: String,
        bs58_signature: String,
        joined_private_commitments: String,
        joined_public_attributes: String,
    ) -> Result<i64, sqlx::Error>;

    /// Attempts to retrieve issued credentials from the data store using provided ids.    
    ///
    /// # Arguments
    ///
    /// * `credential_ids`: (database) ids of the issued credentials
    async fn get_issued_credentials(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<Vec<IssuedCredential>, sqlx::Error>;

    /// Attempts to retrieve issued credentials from the data store using pagination specification.    
    ///
    /// # Arguments
    ///
    /// * `start_after`: the value preceding the first retrieved result
    /// * `limit`: the maximum number of entries to retrieve
    async fn get_issued_credentials_paged(
        &self,
        start_after: i64,
        limit: u32,
    ) -> Result<Vec<IssuedCredential>, sqlx::Error>;

    async fn increment_issued_freepasses(&self) -> Result<(), sqlx::Error>;
}

#[async_trait]
impl CoconutStorageManagerExt for StorageManager {
    /// Gets the information about all issued partial credentials in this (coconut) epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the (coconut) epoch in question.
    async fn get_epoch_credentials(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochCredentials>, sqlx::Error> {
        // even if we were changing epochs every second, it's rather impossible to overflow here
        // within any sane amount of time
        assert!(epoch_id <= u32::MAX as u64);
        let epoch_id_downcasted = epoch_id as u32;

        sqlx::query_as!(
            EpochCredentials,
            r#"
                SELECT epoch_id as "epoch_id: u32", start_id, total_issued as "total_issued: u32"
                FROM epoch_credentials
                WHERE epoch_id = ?
            "#,
            epoch_id_downcasted
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Creates new entry for EpochCredentials for this (coconut) epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the (coconut) epoch in question.
    async fn create_epoch_credentials_entry(&self, epoch_id: EpochId) -> Result<(), sqlx::Error> {
        // even if we were changing epochs every second, it's rather impossible to overflow here
        // within any sane amount of time
        assert!(epoch_id <= u32::MAX as u64);
        let epoch_id_downcasted = epoch_id as u32;

        sqlx::query!(
            r#"
                INSERT INTO epoch_credentials 
                (epoch_id, start_id, total_issued)
                VALUES (?, ?, ?);
            "#,
            epoch_id_downcasted,
            -1,
            0
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    // the logic in this function can be summarised with:
    // 1. get the current EpochCredentials for this epoch
    // 2. if it exists -> increment `total_issued`
    // 3. it has invalid (i.e. -1) `start_id` set it to the provided value
    // 4. if it didn't exist, create new entry
    /// Update the EpochCredentials by incrementing the total number of issued credentials,
    /// and setting `start_id` if unset (i.e. this is the first credential issued this epoch)
    ///
    /// # Arguments
    /// * `epoch_id`: Id of the (coconut) epoch in question.
    /// * `credential_id`: (database) Id of the coconut credential that triggered the update.
    async fn update_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
        credential_id: i64,
    ) -> Result<(), sqlx::Error> {
        // even if we were changing epochs every second, it's rather impossible to overflow here
        // within any sane amount of time
        assert!(epoch_id <= u32::MAX as u64);
        let epoch_id_downcasted = epoch_id as u32;

        // make the atomic transaction in case other tasks are attempting to use the pool
        let mut tx = self.connection_pool.begin().await?;

        if let Some(existing) = sqlx::query_as!(
            EpochCredentials,
            r#"
                SELECT epoch_id as "epoch_id: u32", start_id, total_issued as "total_issued: u32"
                FROM epoch_credentials
                WHERE epoch_id = ?
            "#,
            epoch_id_downcasted
        )
        .fetch_optional(&mut tx)
        .await?
        {
            // the entry has existed before -> update it
            if existing.total_issued == 0 {
                // no credentials has been issued -> we have to set the `start_id`
                sqlx::query!(
                    r#"
                        UPDATE epoch_credentials 
                        SET total_issued = 1, start_id = ?
                        WHERE epoch_id = ?
                    "#,
                    credential_id,
                    epoch_id_downcasted
                )
                .execute(&mut tx)
                .await?;
            } else {
                // we have issued credentials in this epoch before -> just increment `total_issued`
                sqlx::query!(
                    r#"
                        UPDATE epoch_credentials 
                        SET total_issued = total_issued + 1 
                        WHERE epoch_id = ?
                    "#,
                    epoch_id_downcasted
                )
                .execute(&mut tx)
                .await?;
            }
        } else {
            // the entry has never been created -> probably some race condition; create it instead
            sqlx::query!(
                r#"
                    INSERT INTO epoch_credentials 
                    (epoch_id, start_id, total_issued)
                    VALUES (?, ?, ?);
                "#,
                epoch_id_downcasted,
                credential_id,
                1
            )
            .execute(&mut tx)
            .await?;
        }

        // finally commit the transaction
        tx.commit().await
    }

    /// Attempts to retrieve an issued credential from the data store.    
    ///
    /// # Arguments
    ///
    /// * `credential_id`: (database) id of the issued credential
    async fn get_issued_credential(
        &self,
        credential_id: i64,
    ) -> Result<Option<IssuedCredential>, sqlx::Error> {
        sqlx::query_as!(
            IssuedCredential,
            r#"
                SELECT id, epoch_id as "epoch_id: u32", tx_hash, bs58_partial_credential, bs58_signature,joined_private_commitments, joined_public_attributes
                FROM issued_credential
                WHERE id = ?
            "#,
            credential_id
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Attempts to retrieve an issued credential from the data store.
    ///
    /// # Arguments
    ///
    /// * `tx_hash`: transaction hash of the deposit used in the issued bandwidth credential
    async fn get_issued_bandwidth_credential_by_hash(
        &self,
        tx_hash: &str,
    ) -> Result<Option<IssuedCredential>, sqlx::Error> {
        sqlx::query_as!(
            IssuedCredential,
            r#"
                SELECT id, epoch_id as "epoch_id: u32", tx_hash, bs58_partial_credential, bs58_signature,joined_private_commitments, joined_public_attributes
                FROM issued_credential
                WHERE tx_hash = ?
            "#,
            tx_hash
        )
            .fetch_optional(&self.connection_pool)
            .await
    }

    /// Store the provided issued credential information and return its (database) id.
    ///
    /// # Arguments
    ///
    /// * `credential`: partial credential, alongside any data required for verification.
    async fn store_issued_credential(
        &self,
        epoch_id: u32,
        tx_hash: String,
        bs58_partial_credential: String,
        bs58_signature: String,
        joined_private_commitments: String,
        joined_public_attributes: String,
    ) -> Result<i64, sqlx::Error> {
        let row_id = sqlx::query!(
            r#"
                INSERT INTO issued_credential
                (epoch_id, tx_hash, bs58_partial_credential, bs58_signature, joined_private_commitments, joined_public_attributes)
                VALUES
                (?, ?, ?, ?, ?, ?)
            "#,
            epoch_id, tx_hash, bs58_partial_credential, bs58_signature, joined_private_commitments, joined_public_attributes
        ).execute(&self.connection_pool).await?.last_insert_rowid();

        Ok(row_id)
    }

    /// Attempts to retrieve issued credentials from the data store using provided ids.    
    ///
    /// # Arguments
    ///
    /// * `credential_ids`: (database) ids of the issued credentials
    async fn get_issued_credentials(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<Vec<IssuedCredential>, sqlx::Error> {
        // that sucks : (
        // https://stackoverflow.com/a/70032524
        let params = format!("?{}", ", ?".repeat(credential_ids.len() - 1));
        let query_str = format!("SELECT * FROM issued_credential WHERE id IN ( {params} )");
        let mut query = sqlx::query_as(&query_str);
        for id in credential_ids {
            query = query.bind(id)
        }

        query.fetch_all(&self.connection_pool).await
    }

    /// Attempts to retrieve issued credentials from the data store using pagination specification.    
    ///
    /// # Arguments
    ///
    /// * `start_after`: the value preceding the first retrieved result
    /// * `limit`: the maximum number of entries to retrieve
    async fn get_issued_credentials_paged(
        &self,
        start_after: i64,
        limit: u32,
    ) -> Result<Vec<IssuedCredential>, sqlx::Error> {
        sqlx::query_as!(
            IssuedCredential,
            r#"
                SELECT id, epoch_id as "epoch_id: u32", tx_hash, bs58_partial_credential, bs58_signature,joined_private_commitments, joined_public_attributes
                FROM issued_credential
                WHERE id > ?
                ORDER BY id
                LIMIT ?
            "#,
            start_after,
            limit
        )
            .fetch_all(&self.connection_pool)
            .await
    }

    async fn increment_issued_freepasses(&self) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE issued_freepass SET issued = issued + 1",)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }
}
