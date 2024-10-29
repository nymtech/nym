// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::storage::models::{
    EpochCredentials, IssuedHash, IssuedTicketbook, RawExpirationDateSignatures,
    SerialNumberWrapper, StoredBloomfilterParams, TicketProvider, VerifiedTicket,
};
use crate::support::storage::manager::StorageManager;
use async_trait::async_trait;
use nym_coconut_dkg_common::types::EpochId;
use nym_ecash_contract_common::deposit::DepositId;
use time::{Date, OffsetDateTime};
use tracing::error;

#[async_trait]
pub trait EcashStorageManagerExt {
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
    ) -> Result<Option<IssuedTicketbook>, sqlx::Error>;

    /// Attempts to retrieve an issued credential from the data store.
    ///
    /// # Arguments
    ///
    /// * `deposit_id`: id the deposit used in the issued bandwidth credential
    async fn get_issued_bandwidth_credential_by_deposit_id(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<IssuedTicketbook>, sqlx::Error>;

    /// Get the hashes of all issued ticketbooks with the particular expiration date
    async fn get_issued_hashes(
        &self,
        expiration_date: Date,
    ) -> Result<Vec<IssuedHash>, sqlx::Error>;

    /// Store the provided issued credential information.
    #[allow(clippy::too_many_arguments)]
    async fn store_issued_ticketbook(
        &self,
        deposit_id: DepositId,
        dkg_epoch_id: u32,
        blinded_partial_credential: &[u8],
        joined_private_commitments: &[u8],
        expiration_date: Date,
        ticketbook_type_repr: u8,
        merkle_leaf: &[u8],
    ) -> Result<(), sqlx::Error>;

    /// Attempts to retrieve issued credentials from the data store using provided ids.
    ///
    /// # Arguments
    ///
    /// * `credential_ids`: (database) ids of the issued credentials
    async fn get_issued_ticketbooks(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<Vec<IssuedTicketbook>, sqlx::Error>;

    /// Attempts to retrieve issued credentials from the data store using pagination specification.
    ///
    /// # Arguments
    ///
    /// * `start_after`: the value preceding the first retrieved result
    /// * `limit`: the maximum number of entries to retrieve
    async fn get_issued_ticketbooks_paged(
        &self,
        start_after: i64,
        limit: u32,
    ) -> Result<Vec<IssuedTicketbook>, sqlx::Error>;

    async fn insert_ticket_provider(&self, gateway_address: &str) -> Result<i64, sqlx::Error>;

    async fn get_ticket_provider(
        &self,
        gateway_address: &str,
    ) -> Result<Option<TicketProvider>, sqlx::Error>;

    async fn insert_verified_ticket(
        &self,
        provider_id: i64,
        spending_date: Date,
        verified_at: OffsetDateTime,
        ticket_data: Vec<u8>,
        serial_number: Vec<u8>,
    ) -> Result<(), sqlx::Error>;

    async fn get_ticket(&self, serial_number: &[u8])
        -> Result<Option<VerifiedTicket>, sqlx::Error>;

    async fn get_provider_ticket_serial_numbers(
        &self,
        provider_id: i64,
        since: OffsetDateTime,
    ) -> Result<Vec<SerialNumberWrapper>, sqlx::Error>;

    async fn get_spent_tickets_on(
        &self,
        date: Date,
    ) -> Result<Vec<SerialNumberWrapper>, sqlx::Error>;

    async fn get_master_verification_key(
        &self,
        epoch_id: i64,
    ) -> Result<Option<Vec<u8>>, sqlx::Error>;
    async fn insert_master_verification_key(
        &self,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error>;

    async fn get_partial_coin_index_signatures(
        &self,
        epoch_id: i64,
    ) -> Result<Option<Vec<u8>>, sqlx::Error>;
    async fn insert_partial_coin_index_signatures(
        &self,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error>;

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: i64,
    ) -> Result<Option<Vec<u8>>, sqlx::Error>;
    async fn insert_master_coin_index_signatures(
        &self,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error>;

    async fn get_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<RawExpirationDateSignatures>, sqlx::Error>;
    async fn insert_partial_expiration_date_signatures(
        &self,
        epoch_id: i64,
        expiration_date: Date,
        data: &[u8],
    ) -> Result<(), sqlx::Error>;

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<RawExpirationDateSignatures>, sqlx::Error>;
    async fn insert_master_expiration_date_signatures(
        &self,
        epoch_id: i64,
        expiration_date: Date,
        data: &[u8],
    ) -> Result<(), sqlx::Error>;

    async fn insert_double_spending_filter_params(
        &self,
        num_hashes: u32,
        bitmap_size: u32,
        sip0_key0: &[u8],
        sip0_key1: &[u8],
        sip1_key0: &[u8],
        sip1_key1: &[u8],
    ) -> Result<i64, sqlx::Error>;

    async fn get_latest_double_spending_filter_params(
        &self,
    ) -> Result<Option<StoredBloomfilterParams>, sqlx::Error>;

    async fn update_archived_partial_bloomfilter(
        &self,
        date: Date,
        new_bitmap: &[u8],
    ) -> Result<(), sqlx::Error>;

    async fn try_load_partial_bloomfilter_bitmap(
        &self,
        date: Date,
        params_id: i64,
    ) -> Result<Option<Vec<u8>>, sqlx::Error>;

    async fn insert_partial_bloomfilter(
        &self,
        date: Date,
        params_id: i64,
        bitmap: &[u8],
    ) -> Result<(), sqlx::Error>;

    async fn remove_old_partial_bloomfilters(&self, cutoff: Date) -> Result<(), sqlx::Error>;

    async fn remove_expired_verified_tickets(&self, cutoff: Date) -> Result<(), sqlx::Error>;
}

#[async_trait]
impl EcashStorageManagerExt for StorageManager {
    /// Gets the information about all issued partial credentials in this (coconut) epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the (coconut) epoch in question.
    async fn get_epoch_credentials(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochCredentials>, sqlx::Error> {
        todo!()
        //
        // // even if we were changing epochs every second, it's rather impossible to overflow here
        // // within any sane amount of time
        // assert!(epoch_id <= u32::MAX as u64);
        // let epoch_id_downcasted = epoch_id as u32;
        //
        // sqlx::query_as!(
        //     EpochCredentials,
        //     r#"
        //         SELECT epoch_id as "epoch_id: u32", start_id, total_issued as "total_issued: u32"
        //         FROM epoch_credentials
        //         WHERE epoch_id = ?
        //     "#,
        //     epoch_id_downcasted
        // )
        // .fetch_optional(&self.connection_pool)
        // .await
    }

    /// Creates new entry for EpochCredentials for this (coconut) epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the (coconut) epoch in question.
    async fn create_epoch_credentials_entry(&self, epoch_id: EpochId) -> Result<(), sqlx::Error> {
        todo!()
        // // even if we were changing epochs every second, it's rather impossible to overflow here
        // // within any sane amount of time
        // assert!(epoch_id <= u32::MAX as u64);
        // let epoch_id_downcasted = epoch_id as u32;
        //
        // sqlx::query!(
        //     r#"
        //         INSERT INTO epoch_credentials
        //         (epoch_id, start_id, total_issued)
        //         VALUES (?, ?, ?);
        //     "#,
        //     epoch_id_downcasted,
        //     -1,
        //     0
        // )
        // .execute(&self.connection_pool)
        // .await?;
        // Ok(())
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
        todo!()
        // // even if we were changing epochs every second, it's rather impossible to overflow here
        // // within any sane amount of time
        // assert!(epoch_id <= u32::MAX as u64);
        // let epoch_id_downcasted = epoch_id as u32;
        //
        // // make the atomic transaction in case other tasks are attempting to use the pool
        // let mut tx = self.connection_pool.begin().await?;
        //
        // if let Some(existing) = sqlx::query_as!(
        //     EpochCredentials,
        //     r#"
        //         SELECT epoch_id as "epoch_id: u32", start_id, total_issued as "total_issued: u32"
        //         FROM epoch_credentials
        //         WHERE epoch_id = ?
        //     "#,
        //     epoch_id_downcasted
        // )
        // .fetch_optional(&mut *tx)
        // .await?
        // {
        //     // the entry has existed before -> update it
        //     if existing.total_issued == 0 {
        //         // no credentials has been issued -> we have to set the `start_id`
        //         sqlx::query!(
        //             r#"
        //                 UPDATE epoch_credentials
        //                 SET total_issued = 1, start_id = ?
        //                 WHERE epoch_id = ?
        //             "#,
        //             credential_id,
        //             epoch_id_downcasted
        //         )
        //         .execute(&mut *tx)
        //         .await?;
        //     } else {
        //         // we have issued credentials in this epoch before -> just increment `total_issued`
        //         sqlx::query!(
        //             r#"
        //                 UPDATE epoch_credentials
        //                 SET total_issued = total_issued + 1
        //                 WHERE epoch_id = ?
        //             "#,
        //             epoch_id_downcasted
        //         )
        //         .execute(&mut *tx)
        //         .await?;
        //     }
        // } else {
        //     // the entry has never been created -> probably some race condition; create it instead
        //     sqlx::query!(
        //         r#"
        //             INSERT INTO epoch_credentials
        //             (epoch_id, start_id, total_issued)
        //             VALUES (?, ?, ?);
        //         "#,
        //         epoch_id_downcasted,
        //         credential_id,
        //         1
        //     )
        //     .execute(&mut *tx)
        //     .await?;
        // }
        //
        // // finally commit the transaction
        // tx.commit().await
    }

    /// Attempts to retrieve an issued credential from the data store.
    ///
    /// # Arguments
    ///
    /// * `credential_id`: (database) id of the issued credential
    async fn get_issued_credential(
        &self,
        credential_id: i64,
    ) -> Result<Option<IssuedTicketbook>, sqlx::Error> {
        todo!()
        // sqlx::query_as!(
        //     IssuedTicketbook,
        //     r#"
        //         SELECT
        //             id,
        //             epoch_id as "epoch_id: u32",
        //             deposit_id as "deposit_id: DepositId",
        //             partial_credential,
        //             signature,
        //             joined_private_commitments,
        //             expiration_date as "expiration_date: Date",
        //             ticketbook_type_repr as "ticketbook_type_repr: u8"
        //         FROM issued_ticketbook
        //         WHERE id = ?
        //     "#,
        //     credential_id
        // )
        // .fetch_optional(&self.connection_pool)
        // .await
    }

    /// Attempts to retrieve an issued credential from the data store.
    ///
    /// # Arguments
    ///
    /// * `deposit_id`: id the deposit used in the issued bandwidth credential
    async fn get_issued_bandwidth_credential_by_deposit_id(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<IssuedTicketbook>, sqlx::Error> {
        todo!()
        // sqlx::query_as!(
        //     IssuedTicketbook,
        //     r#"
        //         SELECT
        //             id,
        //             epoch_id as "epoch_id: u32",
        //             deposit_id as "deposit_id: DepositId",
        //             partial_credential,
        //             signature,
        //             joined_private_commitments,
        //             expiration_date as "expiration_date: Date",
        //             ticketbook_type_repr as "ticketbook_type_repr: u8"
        //         FROM issued_ticketbook
        //         WHERE deposit_id = ?
        //     "#,
        //     deposit_id
        // )
        // .fetch_optional(&self.connection_pool)
        // .await
    }

    /// Get the hashes of all issued ticketbooks with the particular expiration date
    async fn get_issued_hashes(
        &self,
        expiration_date: Date,
    ) -> Result<Vec<IssuedHash>, sqlx::Error> {
        Ok(sqlx::query!(
            r#"
                SELECT deposit_id as "deposit_id: DepositId", merkle_leaf FROM issued_ticketbook WHERE expiration_date = ?
            "#,
            expiration_date
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .filter_map(|r| r.merkle_leaf.try_into().inspect_err(|_| error!("possible database corruption: one of the stored merkle leaves is not a valid 32byte hash")).ok().map(|merkle_leaf| IssuedHash {
            deposit_id: r.deposit_id,
            merkle_leaf,
        }))
        .collect())
    }

    /// Store the provided issued credential information.
    async fn store_issued_ticketbook(
        &self,
        deposit_id: DepositId,
        dkg_epoch_id: u32,
        blinded_partial_credential: &[u8],
        joined_private_commitments: &[u8],
        expiration_date: Date,
        ticketbook_type_repr: u8,
        merkle_leaf: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO issued_ticketbook (
                    deposit_id,
                    dkg_epoch_id,
                    blinded_partial_credential,
                    joined_private_commitments,
                    expiration_date,
                    ticketbook_type_repr,
                    merkle_leaf
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            deposit_id,
            dkg_epoch_id,
            blinded_partial_credential,
            joined_private_commitments,
            expiration_date,
            ticketbook_type_repr,
            merkle_leaf
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    /// Attempts to retrieve issued credentials from the data store using provided ids.
    ///
    /// # Arguments
    ///
    /// * `credential_ids`: (database) ids of the issued credentials
    async fn get_issued_ticketbooks(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<Vec<IssuedTicketbook>, sqlx::Error> {
        // that sucks : (
        // https://stackoverflow.com/a/70032524
        let params = format!("?{}", ", ?".repeat(credential_ids.len() - 1));
        let query_str = format!("SELECT * FROM issued_ticketbook WHERE id IN ( {params} )");
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
    async fn get_issued_ticketbooks_paged(
        &self,
        start_after: i64,
        limit: u32,
    ) -> Result<Vec<IssuedTicketbook>, sqlx::Error> {
        todo!()
        // sqlx::query_as!(
        //     IssuedTicketbook,
        //     r#"
        //         SELECT
        //             id,
        //             epoch_id as "epoch_id: u32",
        //             deposit_id as "deposit_id: DepositId",
        //             partial_credential,
        //             signature,
        //             joined_private_commitments,
        //             expiration_date as "expiration_date: Date",
        //             ticketbook_type_repr as "ticketbook_type_repr: u8"
        //         FROM issued_ticketbook
        //         WHERE id > ?
        //         ORDER BY id
        //         LIMIT ?
        //     "#,
        //     start_after,
        //     limit
        // )
        // .fetch_all(&self.connection_pool)
        // .await
    }

    async fn insert_ticket_provider(&self, gateway_address: &str) -> Result<i64, sqlx::Error> {
        let id = sqlx::query!(
            "INSERT INTO ticket_providers(gateway_address) VALUES (?)",
            gateway_address
        )
        .execute(&self.connection_pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    async fn get_ticket_provider(
        &self,
        gateway_address: &str,
    ) -> Result<Option<TicketProvider>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM ticket_providers WHERE gateway_address = ?")
            .bind(gateway_address)
            .fetch_optional(&self.connection_pool)
            .await
    }
    async fn insert_verified_ticket(
        &self,
        provider_id: i64,
        spending_date: Date,
        verified_at: OffsetDateTime,
        ticket_data: Vec<u8>,
        serial_number: Vec<u8>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO verified_tickets(ticket_data, serial_number, spending_date, verified_at, gateway_id)
                VALUES (?, ?, ?, ?, ?)
            "#,
            ticket_data,
            serial_number,
            spending_date,
            verified_at,
            provider_id
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    async fn get_ticket(
        &self,
        serial_number: &[u8],
    ) -> Result<Option<VerifiedTicket>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM verified_tickets WHERE serial_number = ?")
            .bind(serial_number)
            .fetch_optional(&self.connection_pool)
            .await
    }

    async fn get_provider_ticket_serial_numbers(
        &self,
        provider_id: i64,
        since: OffsetDateTime,
    ) -> Result<Vec<SerialNumberWrapper>, sqlx::Error> {
        sqlx::query_as!(
            SerialNumberWrapper,
            r#"
                SELECT serial_number
                FROM verified_tickets
                WHERE gateway_id = ?
                AND verified_at > ?
                ORDER BY verified_at ASC
                LIMIT 65535
            "#,
            provider_id,
            since
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    async fn get_spent_tickets_on(
        &self,
        date: Date,
    ) -> Result<Vec<SerialNumberWrapper>, sqlx::Error> {
        sqlx::query_as!(
            SerialNumberWrapper,
            r#"
                SELECT serial_number
                FROM verified_tickets
                WHERE spending_date = ?
            "#,
            date
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    async fn get_master_verification_key(
        &self,
        epoch_id: i64,
    ) -> Result<Option<Vec<u8>>, sqlx::Error> {
        sqlx::query!(
            "SELECT serialised_key FROM master_verification_key WHERE epoch_id = ?",
            epoch_id
        )
        .fetch_optional(&self.connection_pool)
        .await
        .map(|maybe_record| maybe_record.map(|r| r.serialised_key))
    }

    async fn insert_master_verification_key(
        &self,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO master_verification_key(epoch_id, serialised_key) VALUES (?, ?)",
            epoch_id,
            data
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    async fn get_partial_coin_index_signatures(
        &self,
        epoch_id: i64,
    ) -> Result<Option<Vec<u8>>, sqlx::Error> {
        sqlx::query!(
            "SELECT serialised_signatures FROM partial_coin_index_signatures WHERE epoch_id = ?",
            epoch_id
        )
        .fetch_optional(&self.connection_pool)
        .await
        .map(|maybe_record| maybe_record.map(|r| r.serialised_signatures))
    }

    async fn insert_partial_coin_index_signatures(
        &self,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO partial_coin_index_signatures(epoch_id, serialised_signatures) VALUES (?, ?)",
            epoch_id,
            data
        ).execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: i64,
    ) -> Result<Option<Vec<u8>>, sqlx::Error> {
        sqlx::query!(
            "SELECT serialised_signatures FROM global_coin_index_signatures WHERE epoch_id = ?",
            epoch_id
        )
        .fetch_optional(&self.connection_pool)
        .await
        .map(|maybe_record| maybe_record.map(|r| r.serialised_signatures))
    }

    async fn insert_master_coin_index_signatures(
        &self,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO global_coin_index_signatures(epoch_id, serialised_signatures) VALUES (?, ?)",
            epoch_id,
            data
        ).execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    async fn get_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<RawExpirationDateSignatures>, sqlx::Error> {
        sqlx::query_as!(
            RawExpirationDateSignatures,
            r#"
                SELECT epoch_id as "epoch_id: u32", serialised_signatures
                FROM partial_expiration_date_signatures
                WHERE expiration_date = ?
            "#,
            expiration_date
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    async fn insert_partial_expiration_date_signatures(
        &self,
        epoch_id: i64,
        expiration_date: Date,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO partial_expiration_date_signatures(expiration_date, epoch_id, serialised_signatures) VALUES (?, ?, ?)",
            expiration_date,
            epoch_id,
            data
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<RawExpirationDateSignatures>, sqlx::Error> {
        sqlx::query_as!(
            RawExpirationDateSignatures,
            r#"
                SELECT epoch_id as "epoch_id: u32", serialised_signatures
                FROM global_expiration_date_signatures
                WHERE expiration_date = ?
            "#,
            expiration_date
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    async fn insert_master_expiration_date_signatures(
        &self,
        epoch_id: i64,
        expiration_date: Date,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO global_expiration_date_signatures(expiration_date, epoch_id, serialised_signatures) VALUES (?, ?, ?)",
            expiration_date,
            epoch_id,
            data
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    async fn insert_double_spending_filter_params(
        &self,
        num_hashes: u32,
        bitmap_size: u32,
        sip0_key0: &[u8],
        sip0_key1: &[u8],
        sip1_key0: &[u8],
        sip1_key1: &[u8],
    ) -> Result<i64, sqlx::Error> {
        let row_id = sqlx::query!(
            r#"
                INSERT INTO bloomfilter_parameters(num_hashes, bitmap_size,sip0_key0, sip0_key1, sip1_key0, sip1_key1)
                VALUES (?, ?, ?, ?, ?, ?)
            "#,
            num_hashes,
            bitmap_size,
            sip0_key0,
            sip0_key1,
            sip1_key0,
            sip1_key1
        ).execute(&self.connection_pool).await?.last_insert_rowid();
        Ok(row_id)
    }

    async fn get_latest_double_spending_filter_params(
        &self,
    ) -> Result<Option<StoredBloomfilterParams>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM bloomfilter_parameters ORDER BY id DESC LIMIT 1")
            .fetch_optional(&self.connection_pool)
            .await
    }

    async fn update_archived_partial_bloomfilter(
        &self,
        date: Date,
        new_bitmap: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE partial_bloomfilter SET bitmap = ? WHERE date = ?",
            new_bitmap,
            date
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    async fn try_load_partial_bloomfilter_bitmap(
        &self,
        date: Date,
        params_id: i64,
    ) -> Result<Option<Vec<u8>>, sqlx::Error> {
        sqlx::query!(
            "SELECT bitmap FROM partial_bloomfilter WHERE date = ? AND parameters = ?",
            date,
            params_id
        )
        .fetch_optional(&self.connection_pool)
        .await
        .map(|maybe_record| maybe_record.map(|r| r.bitmap))
    }

    async fn insert_partial_bloomfilter(
        &self,
        date: Date,
        params_id: i64,
        bitmap: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO partial_bloomfilter(date, parameters, bitmap) VALUES (?, ?, ?)",
            date,
            params_id,
            bitmap
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    async fn remove_old_partial_bloomfilters(&self, cutoff: Date) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM partial_bloomfilter WHERE date > ?", cutoff)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    async fn remove_expired_verified_tickets(&self, cutoff: Date) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM verified_tickets WHERE spending_date > ?",
            cutoff
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
