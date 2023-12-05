// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::storage::models::EpochCredentials;
use crate::support::storage::manager::StorageManager;
use nym_coconut_dkg_common::types::EpochId;

#[async_trait]
pub trait CoconutStorageManagerExt {
    /// Creates new encrypted blinded signature response entry for a given deposit tx hash.
    ///
    /// # Arguments
    ///
    /// * `tx_hash`: hash of the deposit transaction.
    /// * `blinded_signature_response`: the encrypted blinded signature response.
    #[deprecated]
    async fn insert_blinded_signature_response(
        &self,
        _tx_hash: &str,
        _blinded_signature_response: &str,
    ) -> Result<(), sqlx::Error> {
        Ok(())
    }

    /// Tries to obtain encrypted blinded signature response for a given transaction hash.
    ///
    /// # Arguments
    ///
    /// * `tx_hash`: transaction hash of the deposit.
    #[deprecated]
    async fn get_blinded_signature_response(
        &self,
        _tx_hash: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        Ok(None)
    }

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
        assert!(u32::MAX as u64 <= epoch_id);
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
        assert!(u32::MAX as u64 <= epoch_id);
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
        assert!(u32::MAX as u64 <= epoch_id);
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
            .execute(&self.connection_pool)
            .await?;
        }

        // finally commit the transaction
        tx.commit().await
    }
}
