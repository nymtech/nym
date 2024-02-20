// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::models::{CommitSignature, Validator};
use sqlx::types::time::OffsetDateTime;
use sqlx::{Executor, Sqlite};
use tracing::{instrument, trace};

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

impl StorageManager {
    pub(crate) async fn set_initial_metadata(&self) -> Result<(), sqlx::Error> {
        if sqlx::query("SELECT * from metadata")
            .fetch_optional(&self.connection_pool)
            .await?
            .is_none()
        {
            sqlx::query("INSERT INTO metadata (id, last_processed_height) VALUES (0, 0)")
                .execute(&self.connection_pool)
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn get_first_block_height_after(
        &self,
        time: OffsetDateTime,
    ) -> Result<Option<i64>, sqlx::Error> {
        let maybe_record = sqlx::query!(
            r#"
                SELECT height
                FROM block
                WHERE timestamp > ?
                ORDER BY timestamp
                LIMIT 1
            "#,
            time
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        if let Some(row) = maybe_record {
            Ok(row.height)
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn get_last_block_height_before(
        &self,
        time: OffsetDateTime,
    ) -> Result<Option<i64>, sqlx::Error> {
        let maybe_record = sqlx::query!(
            r#"
                SELECT height
                FROM block
                WHERE timestamp < ?
                ORDER BY timestamp DESC
                LIMIT 1
            "#,
            time
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        if let Some(row) = maybe_record {
            Ok(row.height)
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn get_signed_between(
        &self,
        consensus_address: &str,
        start_height: i64,
        end_height: i64,
    ) -> Result<i32, sqlx::Error> {
        let count = sqlx::query!(
            r#"
                SELECT COUNT(*) as count FROM pre_commit
                WHERE 
                    validator_address == ?
                    AND height >= ? 
                    AND height <= ?
            "#,
            consensus_address,
            start_height,
            end_height
        )
        .fetch_one(&self.connection_pool)
        .await?
        .count;

        Ok(count)
    }

    pub(crate) async fn get_precommit(
        &self,
        consensus_address: &str,
        height: i64,
    ) -> Result<Option<CommitSignature>, sqlx::Error> {
        sqlx::query_as(
            r#"
                SELECT * FROM pre_commit
                WHERE validator_address = ? 
                AND height = ?
            "#,
        )
        .bind(consensus_address)
        .bind(height)
        .fetch_optional(&self.connection_pool)
        .await
    }

    pub(crate) async fn get_block_validators(
        &self,
        height: i64,
    ) -> Result<Vec<Validator>, sqlx::Error> {
        sqlx::query_as!(
            Validator,
            r#"
                SELECT * FROM validator 
                WHERE EXISTS (
                    SELECT 1 FROM pre_commit
                    WHERE height == ?
                    AND pre_commit.validator_address = validator.consensus_address
                )
            "#,
            height
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    pub(crate) async fn get_validators(&self) -> Result<Vec<Validator>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM validator")
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(crate) async fn get_last_processed_height(&self) -> Result<i64, sqlx::Error> {
        let maybe_record = sqlx::query!(
            r#"
                SELECT last_processed_height FROM metadata
            "#
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        if let Some(row) = maybe_record {
            Ok(row.last_processed_height)
        } else {
            Ok(-1)
        }
    }
}

// make those generic over executor so that they could be performed over connection pool and a tx

#[instrument(skip(executor))]
pub(crate) async fn insert_validator<'a, E>(
    consensus_address: String,
    consensus_pubkey: String,
    executor: E,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("insert validator");

    sqlx::query!(
        r#"
            INSERT INTO validator (consensus_address, consensus_pubkey)
            VALUES (?, ?)
            ON CONFLICT DO NOTHING
        "#,
        consensus_address,
        consensus_pubkey
    )
    .execute(executor)
    .await?;

    Ok(())
}

#[instrument(skip(executor))]
pub(crate) async fn insert_block<'a, E>(
    height: i64,
    hash: String,
    num_txs: u32,
    total_gas: i64,
    proposer_address: String,
    timestamp: OffsetDateTime,
    executor: E,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("insert block");

    sqlx::query!(
        r#"
            INSERT INTO block (height, hash, num_txs, total_gas, proposer_address, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT DO NOTHING
        "#,
        height,
        hash,
        num_txs,
        total_gas,
        proposer_address,
        timestamp
    )
    .execute(executor)
    .await?;

    Ok(())
}

#[instrument(skip(executor))]
pub(crate) async fn insert_precommit<'a, E>(
    validator_address: String,
    height: i64,
    timestamp: OffsetDateTime,
    voting_power: i64,
    proposer_priority: i64,
    executor: E,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("insert precommit");

    sqlx::query!(
        r#"
            INSERT INTO pre_commit (validator_address, height, timestamp, voting_power, proposer_priority)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (validator_address, timestamp) DO NOTHING
        "#,
        validator_address,
        height,
        timestamp,
        voting_power,
        proposer_priority
    )
    .execute(executor)
    .await?;

    Ok(())
}

#[instrument(skip(executor))]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_transaction<'a, E>(
    hash: String,
    height: i64,
    index: i64,
    success: bool,
    messages: i64,
    memo: String,
    gas_wanted: i64,
    gas_used: i64,
    raw_log: String,
    executor: E,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("insert transaction");

    sqlx::query!(
        r#"
            INSERT INTO "transaction" (hash, height, "index", success, num_messages, memo, gas_wanted, gas_used, raw_log)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT (hash) DO UPDATE
               SET height = excluded.height,
               "index" = excluded."index",
               success = excluded.success,
               num_messages = excluded.num_messages,
               memo = excluded.memo,
               gas_wanted = excluded.gas_wanted,
               gas_used = excluded.gas_used,
               raw_log = excluded.raw_log
        "#,
            hash,
            height,
            index,
            success,
            messages,
            memo,
            gas_wanted,
            gas_used,
            raw_log,
    )
        .execute(executor)
        .await?;

    Ok(())
}

#[instrument(skip(executor))]
pub(crate) async fn insert_message<'a, E>(
    transaction_hash: String,
    index: i64,
    typ: String,
    height: i64,
    executor: E,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("insert message");

    sqlx::query!(
        r#"
            INSERT INTO message (transaction_hash, "index", type, height)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (transaction_hash, "index") DO UPDATE
                SET height = excluded.height,
                type = excluded.type
        "#,
        transaction_hash,
        index,
        typ,
        height
    )
    .execute(executor)
    .await?;

    Ok(())
}

#[instrument(skip(executor))]
pub(crate) async fn update_last_processed<'a, E>(
    height: i64,
    executor: E,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("update_last_processed");

    sqlx::query!("UPDATE metadata SET last_processed_height = ?", height)
        .execute(executor)
        .await?;

    Ok(())
}
