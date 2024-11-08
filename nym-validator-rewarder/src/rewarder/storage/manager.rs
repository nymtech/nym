// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::rewarder::epoch::Epoch;
use time::{Date, OffsetDateTime};

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

impl StorageManager {
    pub(crate) async fn load_last_block_signing_rewarding_epoch(
        &self,
    ) -> Result<Option<Epoch>, sqlx::Error> {
        sqlx::query_as(
            r#"
                    SELECT id, start_time, end_time
                    FROM block_signing_rewarding_epoch
                    ORDER BY id DESC
                    LIMIT 1
                "#,
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    pub(crate) async fn load_last_ticketbook_issuance_expiration_date(
        &self,
    ) -> Result<Option<Date>, sqlx::Error> {
        Ok(sqlx::query!(
            r#"
                SELECT expiration_date as "expiration_date: Date"
                FROM ticketbook_issuance_epoch
                ORDER BY expiration_date DESC
                LIMIT 1
            "#,
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|record| record.expiration_date))
    }

    pub(crate) async fn load_banned_ticketbook_issuers(&self) -> Result<Vec<String>, sqlx::Error> {
        Ok(sqlx::query!(
            r#"
                SELECT operator_account
                FROM banned_ticketbook_issuer
            "#,
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|record| record.operator_account)
        .collect())
    }

    pub(crate) async fn insert_block_signing_rewarding_epoch(
        &self,
        epoch: Epoch,
        block_signing_budget: String,
        disabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO block_signing_rewarding_epoch (id, start_time, end_time, budget, disabled)
                VALUES (?, ?, ?, ?, ?)
            "#,
            epoch.id,
            epoch.start_time,
            epoch.end_time,
            block_signing_budget,
            disabled
        ).execute(&self.connection_pool).await?;

        Ok(())
    }

    pub(crate) async fn insert_ticketbook_issuance_epoch(
        &self,
        expiration_date: Date,
        total_budget: String,
        whitelist_size: u32,
        budget_per_operator: String,
        disabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO ticketbook_issuance_epoch(
                    expiration_date,
                    total_budget,
                    whitelist_size,
                    budget_per_operator,
                    disabled
                ) VALUES (?, ?, ?, ?, ?)
            "#,
            expiration_date,
            total_budget,
            whitelist_size,
            budget_per_operator,
            disabled
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn insert_block_signing_rewarding_details(
        &self,
        epoch: i64,
        total_voting_power_at_epoch_start: i64,
        num_blocks: i64,
        total_spent: String,
        rewarding_tx: Option<String>,
        rewarding_error: Option<String>,
        monitor_only: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO block_signing_rewarding_details(
                    rewarding_epoch_id,
                    total_voting_power_at_epoch_start,
                    num_blocks,
                    spent,
                    rewarding_tx,
                    rewarding_error,
                    monitor_only
               ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            epoch,
            total_voting_power_at_epoch_start,
            num_blocks,
            total_spent,
            rewarding_tx,
            rewarding_error,
            monitor_only
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_ticketbook_issuance_rewarding_details(
        &self,
        ticketbook_expiration_date: Date,
        approximate_deposits: i64,
        total_spent: String,
        rewarding_tx: Option<String>,
        rewarding_error: Option<String>,
        monitor_only: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO ticketbook_issuance_rewarding_details(
                    ticketbook_expiration_date,
                    approximate_deposits,
                    spent,
                    rewarding_tx,
                    rewarding_error,
                    monitor_only
                ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
            ticketbook_expiration_date,
            approximate_deposits,
            total_spent,
            rewarding_tx,
            rewarding_error,
            monitor_only
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn insert_block_signing_reward(
        &self,
        epoch: i64,
        consensus_address: String,
        operator_account: String,
        whitelisted: bool,
        amount: String,
        voting_power: i64,
        voting_power_share: String,
        signed_blocks: i32,
        signed_blocks_percent: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO block_signing_reward (
                    rewarding_epoch_id,
                    validator_consensus_address,
                    operator_account,
                    whitelisted,
                    amount,
                    voting_power,
                    voting_power_share,
                    signed_blocks,
                    signed_blocks_percent
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            epoch,
            consensus_address,
            operator_account,
            whitelisted,
            amount,
            voting_power,
            voting_power_share,
            signed_blocks,
            signed_blocks_percent,
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn insert_ticketbook_issuance_reward(
        &self,
        ticketbook_expiration_date: Date,
        api_endpoint: String,
        operator_account: String,
        whitelisted: bool,
        banned: bool,
        amount: String,
        issued_partial_ticketbooks: u32,
        share_of_issued_ticketbooks: f32,
        skipped_verification: bool,
        subsample_size: u32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO ticketbook_issuance_reward(
                    ticketbook_expiration_date,
                    api_endpoint,
                    operator_account,
                    whitelisted,
                    banned,
                    amount,
                    issued_partial_ticketbooks,
                    share_of_issued_ticketbooks,
                    skipped_verification,
                    subsample_size
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            ticketbook_expiration_date,
            api_endpoint,
            operator_account,
            whitelisted,
            banned,
            amount,
            issued_partial_ticketbooks,
            share_of_issued_ticketbooks,
            skipped_verification,
            subsample_size,
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_banned_ticketbook_issuer(
        &self,
        operator_account: String,
        api_endpoint: String,
        banned_on: OffsetDateTime,
        associated_ticketbook_expiration_date: Date,
        reason: String,
        evidence: Vec<u8>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO banned_ticketbook_issuer(
                    operator_account,
                    api_endpoint,
                    banned_on,
                    associated_ticketbook_expiration_date,
                    reason,
                    evidence
                ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
            operator_account,
            api_endpoint,
            banned_on,
            associated_ticketbook_expiration_date,
            reason,
            evidence,
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }
}
