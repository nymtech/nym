// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{
    BasicTicketbookInformation, RawCoinIndexSignatures, RawExpirationDateSignatures,
    RawVerificationKey, StoredIssuedTicketbook, StoredPendingTicketbook,
};
use nym_ecash_time::Date;
use sqlx::{Executor, Sqlite, Transaction};

#[derive(Clone)]
pub struct SqliteEcashTicketbookManager {
    connection_pool: sqlx::SqlitePool,
}

impl SqliteEcashTicketbookManager {
    /// Creates new instance of the `EcashTicketbookManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub fn new(connection_pool: sqlx::SqlitePool) -> Self {
        SqliteEcashTicketbookManager { connection_pool }
    }

    pub(crate) async fn cleanup_expired(&self, deadline: Date) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM ecash_ticketbook WHERE expiration_date <= ?",
            deadline
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn begin_storage_tx(&self) -> Result<Transaction<Sqlite>, sqlx::Error> {
        self.connection_pool.begin().await
    }

    pub(crate) async fn insert_pending_ticketbook(
        &self,
        serialisation_revision: u8,
        deposit_id: u32,
        data: &[u8],
        expiration_date: Date,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO pending_issuance
                (deposit_id, serialization_revision, pending_ticketbook_data, expiration_date)
                VALUES (?, ?, ?, ?)
            "#,
            deposit_id,
            serialisation_revision,
            data,
            expiration_date,
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn insert_new_ticketbook(
        &self,
        serialisation_revision: u8,
        data: &[u8],
        expiration_date: Date,
        typ: &str,
        epoch_id: u32,
        total_tickets: u32,
        used_tickets: u32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO ecash_ticketbook
                (serialization_revision, ticketbook_data, expiration_date, ticketbook_type, epoch_id, total_tickets, used_tickets)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            serialisation_revision,
            data,
            expiration_date,
            typ,
            epoch_id,
            total_tickets,
            used_tickets,
        ).execute(&self.connection_pool).await?;

        Ok(())
    }

    pub(crate) async fn get_ticketbooks_info(
        &self,
    ) -> Result<Vec<BasicTicketbookInformation>, sqlx::Error> {
        sqlx::query_as(
            r#"
                    SELECT id, expiration_date, ticketbook_type, epoch_id, total_tickets, used_tickets
                    FROM ecash_ticketbook
                "#,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    pub(crate) async fn decrease_used_ticketbook_tickets(
        &self,
        ticketbook_id: i64,
        reverted_spent: u32,
        expected_current_total_spent: u32,
    ) -> Result<bool, sqlx::Error> {
        // the 'AND' clause will ensure this will only be executed if nobody else interacted with the row
        let affected = sqlx::query!(
            r#"
                UPDATE ecash_ticketbook
                SET used_tickets = used_tickets - ?
                WHERE id = ?
                AND used_tickets = ?
            "#,
            reverted_spent,
            ticketbook_id,
            expected_current_total_spent
        )
        .execute(&self.connection_pool)
        .await?
        .rows_affected();
        Ok(affected > 0)
    }

    pub(crate) async fn get_pending_ticketbooks(
        &self,
    ) -> Result<Vec<StoredPendingTicketbook>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM pending_issuance")
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(crate) async fn remove_pending_ticketbook(
        &self,
        pending_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM pending_issuance WHERE deposit_id = ?",
            pending_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_master_verification_key(
        &self,
        epoch_id: i64,
    ) -> Result<Option<RawVerificationKey>, sqlx::Error> {
        sqlx::query_as!(
            RawVerificationKey,
            r#"
                SELECT epoch_id as "epoch_id: u32", serialised_key, serialization_revision as "serialization_revision: u8"
                FROM master_verification_key WHERE epoch_id = ?
            "#,
            epoch_id
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    pub(crate) async fn insert_master_verification_key(
        &self,
        serialisation_revision: u8,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO master_verification_key(epoch_id, serialised_key, serialization_revision) VALUES (?, ?, ?)",
            epoch_id,
            data,
            serialisation_revision
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_coin_index_signatures(
        &self,
        epoch_id: i64,
    ) -> Result<Option<RawCoinIndexSignatures>, sqlx::Error> {
        sqlx::query_as!(
            RawCoinIndexSignatures,
            r#"
                SELECT epoch_id as "epoch_id: u32", serialised_signatures, serialization_revision as "serialization_revision: u8"
                FROM coin_indices_signatures WHERE epoch_id = ?
            "#,
            epoch_id
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    pub(crate) async fn insert_coin_index_signatures(
        &self,
        serialisation_revision: u8,
        epoch_id: i64,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO coin_indices_signatures(epoch_id, serialised_signatures, serialization_revision) VALUES (?, ?, ?)",
            epoch_id,
            data,
            serialisation_revision
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<RawExpirationDateSignatures>, sqlx::Error> {
        sqlx::query_as!(
            RawExpirationDateSignatures,
            r#"
                SELECT epoch_id as "epoch_id: u32", serialised_signatures, serialization_revision as "serialization_revision: u8"
                FROM expiration_date_signatures
                WHERE expiration_date = ?
            "#,
            expiration_date
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    pub(crate) async fn insert_expiration_date_signatures(
        &self,
        serialisation_revision: u8,
        epoch_id: i64,
        expiration_date: Date,
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO expiration_date_signatures(expiration_date, epoch_id, serialised_signatures, serialization_revision)
                VALUES (?, ?, ?, ?)
            "#,
            expiration_date,
            epoch_id,
            data,
            serialisation_revision
        )
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }
}

pub(crate) async fn get_next_unspent_ticketbook<'a, E>(
    executor: E,
    ticketbook_type: String,
    deadline: Date,
    tickets: u32,
) -> Result<Option<StoredIssuedTicketbook>, sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    sqlx::query_as(
        r#"
                SELECT *
                FROM ecash_ticketbook
                WHERE used_tickets + ? <= total_tickets
                AND expiration_date >= ?
                AND ticketbook_type = ?
                ORDER BY expiration_date ASC
                LIMIT 1
            "#,
    )
    .bind(tickets)
    .bind(deadline)
    .bind(ticketbook_type)
    .fetch_optional(executor)
    .await
}

pub(crate) async fn increase_used_ticketbook_tickets<'a, E>(
    executor: E,
    ticketbook_id: i64,
    extra_spent: u32,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    sqlx::query!(
        "UPDATE ecash_ticketbook SET used_tickets = used_tickets + ? WHERE id = ?",
        extra_spent,
        ticketbook_id
    )
    .execute(executor)
    .await?;
    Ok(())
}
