// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ecash_time::Date;

use nym_sqlx_pool_guard::SqlitePoolGuard;

use crate::storage::models::StoredPendingTicketbook;

#[derive(Clone)]
pub struct SqliteZkNymRequestsStorageManager {
    connection_pool: SqlitePoolGuard,
}

impl SqliteZkNymRequestsStorageManager {
    pub fn new(connection_pool: SqlitePoolGuard) -> Self {
        Self { connection_pool }
    }

    pub async fn close(&self) {
        self.connection_pool.close().await
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
        .execute(&*self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn get_pending_ticketbooks(
        &self,
    ) -> Result<Vec<StoredPendingTicketbook>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM pending_issuance")
            .fetch_all(&*self.connection_pool)
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
        .execute(&*self.connection_pool)
        .await?;
        Ok(())
    }
}
