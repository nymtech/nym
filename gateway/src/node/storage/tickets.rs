// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::models::{RedemptionProposal, VerifiedTicket};
use time::OffsetDateTime;

#[derive(Clone)]
pub(crate) struct TicketStorageManager {
    connection_pool: sqlx::SqlitePool,
}

impl TicketStorageManager {
    /// Creates new instance of the `CredentialManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        TicketStorageManager { connection_pool }
    }

    pub(crate) async fn insert_ecash_signers(
        &self,
        epoch_id: i64,
        signer_ids: Vec<i64>,
    ) -> Result<(), sqlx::Error> {
        let mut query_builder =
            sqlx::QueryBuilder::new("INSERT INTO ecash_signer (epoch_id, signer_id) ");

        query_builder.push_values(signer_ids, |mut b, signer_id| {
            b.push_bind(epoch_id).push_bind(signer_id);
        });

        query_builder.build().execute(&self.connection_pool).await?;
        Ok(())
    }

    pub(crate) async fn insert_new_ticket(
        &self,
        client_id: i64,
        received_at: OffsetDateTime,
    ) -> Result<i64, sqlx::Error> {
        Ok(sqlx::query!(
            "INSERT INTO received_ticket (client_id, received_at) VALUES (?, ?)",
            client_id,
            received_at
        )
        .execute(&self.connection_pool)
        .await?
        .last_insert_rowid())
    }

    pub(crate) async fn set_rejected_ticket(&self, ticket_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE received_ticket SET rejected = true WHERE id = ?",
            ticket_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn insert_ticket_data(
        &self,
        ticket_id: i64,
        serial_number: &[u8],
        data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO ticket_data(ticket_id, serial_number, data) VALUES (?, ?, ?)",
            ticket_id,
            serial_number,
            data
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn remove_ticket_data(&self, ticket_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM ticket_data WHERE ticket_id = ?", ticket_id)
            .execute(&self.connection_pool)
            .await?;

        Ok(())
    }

    pub(crate) async fn has_ticket_data(&self, serial_number: &[u8]) -> Result<bool, sqlx::Error> {
        sqlx::query!(
            "SELECT EXISTS (SELECT 1 FROM ticket_data WHERE serial_number = ?) AS 'exists'",
            serial_number
        )
        .fetch_one(&self.connection_pool)
        .await
        .map(|result| result.exists == 1)
    }

    pub(crate) async fn remove_binary_ticket_data(
        &self,
        ticket_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE ticket_data SET data = NULL WHERE ticket_id = ?",
            ticket_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn remove_redeemed_tickets_data(
        &self,
        proposal_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                DELETE FROM ticket_data
                WHERE ticket_id IN (
                    SELECT ticket_id
                    FROM verified_tickets
                    WHERE proposal_id = ?
                )
            "#,
            proposal_id
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_ticket_verification(
        &self,
        ticket_id: i64,
        signer_id: i64,
        verified_at: OffsetDateTime,
        accepted: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO ticket_verification (ticket_id, signer_id, verified_at, accepted)
                VALUES (?, ?, ?, ?)
            "#,
            ticket_id,
            signer_id,
            verified_at,
            accepted
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn remove_ticket_verification(
        &self,
        ticket_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM ticket_verification WHERE ticket_id = ?",
            ticket_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn insert_verified_ticket(&self, ticket_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO verified_tickets (ticket_id) VALUES (?)",
            ticket_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_verified_tickets_with_sn(
        &self,
    ) -> Result<Vec<VerifiedTicket>, sqlx::Error> {
        sqlx::query_as!(
            VerifiedTicket,
            r#"
                SELECT t1.ticket_id, t2.serial_number
                    FROM verified_tickets as t1
                JOIN ticket_data as t2
                    ON t1.ticket_id = t2.ticket_id
                JOIN received_ticket as t3
                    ON t1.ticket_id = t3.id

                ORDER BY t3.received_at ASC
                LIMIT 65535
        "#
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// for each ticket in `verified_tickets` where the `ticket_id` is present in the provided iterator,
    /// set the associated `proposal_id` to the provided value.
    pub(crate) async fn insert_verified_tickets_proposal_id<I>(
        &self,
        ticket_ids: I,
        proposal_id: i64,
    ) -> Result<(), sqlx::Error>
    where
        I: Iterator<Item = i64>,
    {
        // UPDATE verified_tickets SET proposal_id = ... WHERE ticket_id IN (1,2,3,...)
        let mut query_builder =
            sqlx::QueryBuilder::new("UPDATE verified_tickets SET proposal_id = ");
        query_builder
            .push_bind(proposal_id)
            .push("WHERE ticket_id IN (");

        let mut separated = query_builder.separated(", ");
        for ticket_id in ticket_ids {
            separated.push_bind(ticket_id);
        }
        separated.push_unseparated(") ");

        query_builder.build().execute(&self.connection_pool).await?;
        Ok(())
    }

    pub(crate) async fn remove_verified_tickets(
        &self,
        proposal_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM verified_tickets WHERE proposal_id = ?",
            proposal_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn insert_redemption_proposal(
        &self,
        proposal_id: i64,
        created_at: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO redemption_proposals (proposal_id, created_at) VALUES (?, ?)",
            proposal_id,
            created_at
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn update_redemption_proposal(
        &self,
        proposal_id: i64,
        resolved_at: OffsetDateTime,
        rejected: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE redemption_proposals SET resolved_at = ?, rejected = ? WHERE proposal_id = ?",
            resolved_at,
            rejected,
            proposal_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_latest_redemption_proposal(
        &self,
    ) -> Result<Option<RedemptionProposal>, sqlx::Error> {
        sqlx::query_as(
            r#"
                    SELECT proposal_id, created_at
                    FROM redemption_proposals
                    ORDER BY created_at DESC
                    LIMIT 1
                "#,
        )
        .fetch_optional(&self.connection_pool)
        .await
    }
}
