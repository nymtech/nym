// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::storage::models::{
    IssuedHash, IssuedTicketbooksCount, IssuedTicketbooksForCount, IssuedTicketbooksOnCount,
    RawExpirationDateSignatures, RawIssuedTicketbook, SerialNumberWrapper, TicketProvider,
    VerifiedTicket,
};
use crate::support::storage::manager::StorageManager;
use async_trait::async_trait;
use nym_ecash_contract_common::deposit::DepositId;
use time::{Date, OffsetDateTime};
use tracing::{error, info};

#[async_trait]
pub trait EcashStorageManagerExt {
    /// Attempts to retrieve an issued credential from the data store.
    ///
    /// # Arguments
    ///
    /// * `deposit_id`: id the deposit used in the issued bandwidth credential
    async fn get_issued_partial_signature(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<Vec<u8>>, sqlx::Error>;

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
        merkle_index: u32,
    ) -> Result<(), sqlx::Error>;

    async fn remove_old_issued_ticketbooks(
        &self,
        cutoff_expiration_date: Date,
    ) -> Result<(), sqlx::Error>;

    /// Attempts to retrieve issued ticketbooks from the data store using associated deposits.
    ///
    /// # Arguments
    ///
    /// * `deposit_ids`: deposits used for obtaining underlying ticketbook
    async fn get_issued_ticketbooks(
        &self,
        deposits: &[DepositId],
    ) -> Result<Vec<RawIssuedTicketbook>, sqlx::Error>;

    async fn insert_ticket_provider(&self, gateway_address: &str) -> Result<i64, sqlx::Error>;

    async fn get_ticket_provider(
        &self,
        gateway_address: &str,
    ) -> Result<Option<TicketProvider>, sqlx::Error>;

    /// Returns a boolean to indicate whether the ticket has actually been inserted
    async fn insert_verified_ticket(
        &self,
        provider_id: i64,
        spending_date: Date,
        verified_at: OffsetDateTime,
        ticket_data: Vec<u8>,
        serial_number: Vec<u8>,
    ) -> Result<bool, sqlx::Error>;

    async fn get_ticket(&self, serial_number: &[u8])
        -> Result<Option<VerifiedTicket>, sqlx::Error>;

    async fn get_provider_ticket_serial_numbers(
        &self,
        provider_id: i64,
        since: OffsetDateTime,
    ) -> Result<Vec<SerialNumberWrapper>, sqlx::Error>;
    async fn update_last_batch_verification(
        &self,
        provider_id: i64,
        last_batch_verification: OffsetDateTime,
    ) -> Result<(), sqlx::Error>;

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

    async fn remove_expired_verified_tickets(&self, cutoff: Date) -> Result<(), sqlx::Error>;

    async fn get_issued_ticketbooks_count(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<IssuedTicketbooksCount>, sqlx::Error>;

    async fn get_issued_ticketbooks_on_count(
        &self,
        issuance_date: Date,
    ) -> Result<Vec<IssuedTicketbooksOnCount>, sqlx::Error>;

    async fn get_issued_ticketbooks_for_count(
        &self,
        expiration_date: Date,
    ) -> Result<Vec<IssuedTicketbooksForCount>, sqlx::Error>;
}

#[async_trait]
impl EcashStorageManagerExt for StorageManager {
    /// Attempts to retrieve an issued credential from the data store.
    ///
    /// # Arguments
    ///
    /// * `deposit_id`: id the deposit used in the issued bandwidth credential
    async fn get_issued_partial_signature(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<Vec<u8>>, sqlx::Error> {
        Ok(sqlx::query!(
            r#"
                SELECT
                blinded_partial_credential
                FROM issued_ticketbook
                WHERE deposit_id = ?
            "#,
            deposit_id
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|r| r.blinded_partial_credential))
    }

    /// Get the hashes of all issued ticketbooks with the particular expiration date
    async fn get_issued_hashes(
        &self,
        expiration_date: Date,
    ) -> Result<Vec<IssuedHash>, sqlx::Error> {
        Ok(sqlx::query!(
            r#"
                SELECT deposit_id as "deposit_id: DepositId", merkle_leaf, merkle_index as "merkle_index: u32"
                FROM issued_ticketbook WHERE expiration_date = ?
            "#,
            expiration_date
        )
            .fetch_all(&self.connection_pool)
            .await?
            .into_iter()
            .filter_map(|r| r.merkle_leaf.try_into().inspect_err(|_| error!("possible database corruption: one of the stored merkle leaves is not a valid 32byte hash")).ok().map(|merkle_leaf| IssuedHash {
                deposit_id: r.deposit_id,
                merkle_leaf,
                merkle_index: r.merkle_index as usize,
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
        merkle_index: u32,
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
                    merkle_leaf,
                    merkle_index
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?);

                INSERT INTO issued_ticketbooks_count(expiration_date, count)
                VALUES (?, 1)
                ON CONFLICT(issuance_date, expiration_date) DO UPDATE SET count = count + 1;
            "#,
            deposit_id,
            dkg_epoch_id,
            blinded_partial_credential,
            joined_private_commitments,
            expiration_date,
            ticketbook_type_repr,
            merkle_leaf,
            merkle_index,
            expiration_date
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    async fn remove_old_issued_ticketbooks(
        &self,
        cutoff_expiration_date: Date,
    ) -> Result<(), sqlx::Error> {
        let res = sqlx::query!(
            r#"
                DELETE FROM issued_ticketbook
                WHERE expiration_date < ?
            "#,
            cutoff_expiration_date
        )
        .execute(&self.connection_pool)
        .await?;

        info!("removed {} issued ticketbooks", res.rows_affected());
        Ok(())
    }

    /// Attempts to retrieve issued ticketbooks from the data store using associated deposits.
    ///
    /// # Arguments
    ///
    /// * `deposit_ids`: deposits used for obtaining underlying ticketbook
    async fn get_issued_ticketbooks(
        &self,
        deposits: &[DepositId],
    ) -> Result<Vec<RawIssuedTicketbook>, sqlx::Error> {
        // that sucks : (
        // https://stackoverflow.com/a/70032524

        // NOTE: whilst there's no explicit `LIMIT` here,
        // the API invoking this method forbids using lists of deposits with too many values
        let params = format!("?{}", ", ?".repeat(deposits.len() - 1));
        let query_str = format!("SELECT * FROM issued_ticketbook WHERE deposit_id IN ( {params} )");
        let mut query = sqlx::query_as(&query_str);
        for deposit_id in deposits {
            query = query.bind(deposit_id)
        }

        query.fetch_all(&self.connection_pool).await
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

    /// Returns a boolean to indicate whether the ticket has actually been inserted
    async fn insert_verified_ticket(
        &self,
        provider_id: i64,
        spending_date: Date,
        verified_at: OffsetDateTime,
        ticket_data: Vec<u8>,
        serial_number: Vec<u8>,
    ) -> Result<bool, sqlx::Error> {
        let affected = sqlx::query!(
            r#"
                INSERT OR IGNORE INTO verified_tickets(ticket_data, serial_number, spending_date, verified_at, gateway_id)
                VALUES (?, ?, ?, ?, ?)
            "#,
            ticket_data,
            serial_number,
            spending_date,
            verified_at,
            provider_id
        )
            .execute(&self.connection_pool)
            .await?.rows_affected();

        Ok(affected == 1)
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

    async fn update_last_batch_verification(
        &self,
        provider_id: i64,
        last_batch_verification: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE ticket_providers
                SET last_batch_verification = ?
                WHERE id = ?
            "#,
            last_batch_verification,
            provider_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
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

    async fn remove_expired_verified_tickets(&self, cutoff: Date) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM verified_tickets WHERE spending_date < ?",
            cutoff
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    async fn get_issued_ticketbooks_count(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<IssuedTicketbooksCount>, sqlx::Error> {
        sqlx::query_as!(
            IssuedTicketbooksCount,
            "SELECT issuance_date, expiration_date, count AS 'count: u32' FROM issued_ticketbooks_count LIMIT ? OFFSET ?",
            limit,
            offset
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    async fn get_issued_ticketbooks_on_count(
        &self,
        issuance_date: Date,
    ) -> Result<Vec<IssuedTicketbooksOnCount>, sqlx::Error> {
        sqlx::query_as!(
            IssuedTicketbooksOnCount,
            "SELECT expiration_date, count AS 'count: u32' FROM issued_ticketbooks_count WHERE issuance_date = ?",
            issuance_date
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    async fn get_issued_ticketbooks_for_count(
        &self,
        expiration_date: Date,
    ) -> Result<Vec<IssuedTicketbooksForCount>, sqlx::Error> {
        sqlx::query_as!(
            IssuedTicketbooksForCount,
            "SELECT issuance_date, count AS 'count: u32' FROM issued_ticketbooks_count WHERE expiration_date = ?",
            expiration_date
        )
        .fetch_all(&self.connection_pool)
        .await
    }
}
