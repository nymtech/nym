// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::log_db_operation_time;
use crate::storage::models::{CommitSignature, Validator};
use sqlx::types::time::OffsetDateTime;
use sqlx::{Executor, Sqlite};
use tokio::time::Instant;
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

    pub(crate) async fn get_lowest_block(&self) -> Result<Option<i64>, sqlx::Error> {
        trace!("get_lowest_block");
        let start = Instant::now();

        let maybe_record = sqlx::query!(
            r#"
                SELECT height
                FROM block
                ORDER BY height ASC
                LIMIT 1
            "#,
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        log_db_operation_time("get_lowest_block", start);

        if let Some(row) = maybe_record {
            Ok(row.height)
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn get_first_block_height_after(
        &self,
        time: OffsetDateTime,
    ) -> Result<Option<i64>, sqlx::Error> {
        trace!("get_first_block_height_after");
        let start = Instant::now();

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
        log_db_operation_time("get_first_block_height_after", start);

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
        trace!("get_last_block_height_before");
        let start = Instant::now();

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
        log_db_operation_time("get_last_block_height_before", start);

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
        trace!("get_signed_between");
        let start = Instant::now();

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
        log_db_operation_time("get_signed_between", start);

        Ok(count)
    }

    pub(crate) async fn get_precommit(
        &self,
        consensus_address: &str,
        height: i64,
    ) -> Result<Option<CommitSignature>, sqlx::Error> {
        trace!("get_precommit");
        let start = Instant::now();

        let res = sqlx::query_as(
            r#"
                SELECT * FROM pre_commit
                WHERE validator_address = ? 
                AND height = ?
            "#,
        )
        .bind(consensus_address)
        .bind(height)
        .fetch_optional(&self.connection_pool)
        .await?;
        log_db_operation_time("get_precommit", start);

        Ok(res)
    }

    pub(crate) async fn get_block_validators(
        &self,
        height: i64,
    ) -> Result<Vec<Validator>, sqlx::Error> {
        trace!("get_block_validators");
        let start = Instant::now();

        let res = sqlx::query_as!(
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
        .await?;
        log_db_operation_time("get_block_validators", start);

        Ok(res)
    }

    pub(crate) async fn get_validators(&self) -> Result<Vec<Validator>, sqlx::Error> {
        trace!("get_validators");
        let start = Instant::now();

        let res = sqlx::query_as("SELECT * FROM validator")
            .fetch_all(&self.connection_pool)
            .await?;
        log_db_operation_time("get_validators", start);

        Ok(res)
    }

    pub(crate) async fn get_last_processed_height(&self) -> Result<i64, sqlx::Error> {
        trace!("get_last_processed_height");
        let start = Instant::now();

        let maybe_record = sqlx::query!(
            r#"
                SELECT last_processed_height FROM metadata
            "#
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        log_db_operation_time("get_last_processed_height", start);

        if let Some(row) = maybe_record {
            Ok(row.last_processed_height)
        } else {
            Ok(-1)
        }
    }

    pub(crate) async fn get_pruned_height(&self) -> Result<i64, sqlx::Error> {
        trace!("get_pruned_height");
        let start = Instant::now();

        let maybe_record = sqlx::query!(
            r#"
                SELECT last_pruned_height FROM pruning
            "#
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        log_db_operation_time("get_pruned_height", start);

        if let Some(row) = maybe_record {
            Ok(row.last_pruned_height)
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
    trace!("insert_validator");
    let start = Instant::now();

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
    log_db_operation_time("insert_validator", start);

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
    trace!("insert_block");
    let start = Instant::now();

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
    log_db_operation_time("insert_block", start);

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
    trace!("insert_precommit");
    let start = Instant::now();

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
    log_db_operation_time("insert_precommit", start);

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
    trace!("insert_transaction");
    let start = Instant::now();

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
    log_db_operation_time("insert_transaction", start);

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
    trace!("insert_message");
    let start = Instant::now();

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
    log_db_operation_time("insert_message", start);

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
    let start = Instant::now();

    sqlx::query!("UPDATE metadata SET last_processed_height = MAX(last_processed_height, ?)", height)
        .execute(executor)
        .await?;
    log_db_operation_time("update_last_processed", start);

    Ok(())
}

#[instrument(skip(executor))]
pub(crate) async fn update_last_pruned<'a, E>(height: i64, executor: E) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("update_last_pruned");
    let start = Instant::now();

    sqlx::query!("UPDATE pruning SET last_pruned_height = ?", height)
        .execute(executor)
        .await?;
    log_db_operation_time("update_last_pruned", start);

    Ok(())
}

pub(crate) async fn prune_blocks<'a, E>(oldest_to_keep: i64, executor: E) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("prune_blocks");
    let start = Instant::now();

    sqlx::query!("DELETE FROM block WHERE height < ?", oldest_to_keep)
        .execute(executor)
        .await?;
    log_db_operation_time("prune_blocks", start);

    Ok(())
}

pub(crate) async fn prune_pre_commits<'a, E>(
    oldest_to_keep: i64,
    executor: E,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("prune_pre_commits");
    let start = Instant::now();

    sqlx::query!("DELETE FROM pre_commit WHERE height < ?", oldest_to_keep)
        .execute(executor)
        .await?;
    log_db_operation_time("prune_pre_commits", start);

    Ok(())
}

pub(crate) async fn prune_transactions<'a, E>(
    oldest_to_keep: i64,
    executor: E,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("prune_transactions");
    let start = Instant::now();

    sqlx::query!(
        "DELETE FROM \"transaction\" WHERE height < ?",
        oldest_to_keep
    )
    .execute(executor)
    .await?;
    log_db_operation_time("prune_transactions", start);

    Ok(())
}

pub(crate) async fn prune_messages<'a, E>(
    oldest_to_keep: i64,
    executor: E,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Sqlite>,
{
    trace!("prune_messages");
    let start = Instant::now();

    sqlx::query!("DELETE FROM message WHERE height < ?", oldest_to_keep)
        .execute(executor)
        .await?;
    log_db_operation_time("prune_messages", start);

    Ok(())
}
