// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_processor::types::{FullBlockInformation, ParsedTransactionResponse},
    error::ScraperError,
    storage::{
        manager::{
            insert_block, insert_message, insert_precommit, insert_transaction, insert_validator,
            prune_blocks, prune_messages, prune_pre_commits, prune_transactions,
            update_last_processed, update_last_pruned, StorageManager,
        },
        models::{CommitSignature, Validator},
    },
};
use sqlx::{types::time::OffsetDateTime, ConnectOptions, Sqlite, Transaction};
use std::{fmt::Debug, path::Path};
use tendermint::{
    block::{Commit, CommitSig},
    Block,
};
use tendermint_rpc::endpoint::validators;
use tokio::time::Instant;
use tracing::{debug, error, info, instrument, trace, warn};

mod helpers;
mod manager;
pub mod models;

pub type StorageTransaction = Transaction<'static, Sqlite>;

#[derive(Clone)]
pub struct ScraperStorage {
    pub(crate) manager: StorageManager,
}

pub(crate) fn log_db_operation_time(op_name: &str, start_time: Instant) {
    let elapsed = start_time.elapsed();
    let formatted = humantime::format_duration(elapsed);

    match elapsed.as_millis() {
        v if v > 10000 => error!("{op_name} took {formatted} to execute"),
        v if v > 1000 => warn!("{op_name} took {formatted} to execute"),
        v if v > 100 => info!("{op_name} took {formatted} to execute"),
        v if v > 10 => debug!("{op_name} took {formatted} to execute"),
        _ => trace!("{op_name} took {formatted} to execute"),
    }
}

impl ScraperStorage {
    #[instrument]
    pub async fn init<P: AsRef<Path> + Debug>(database_path: P) -> Result<Self, ScraperError> {
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true)
            .disable_statement_logging();

        // TODO: do we want auto_vacuum ?

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./sql_migrations")
            .run(&connection_pool)
            .await
        {
            error!("Failed to initialize SQLx database: {err}");
            return Err(err.into());
        }

        info!("Database migration finished!");

        let manager = StorageManager { connection_pool };
        manager.set_initial_metadata().await?;

        let storage = ScraperStorage { manager };

        Ok(storage)
    }

    #[instrument(skip(self))]
    pub async fn prune_storage(
        &self,
        oldest_to_keep: u32,
        current_height: u32,
    ) -> Result<(), ScraperError> {
        let start = Instant::now();

        let mut tx = self.begin_processing_tx().await?;

        prune_messages(oldest_to_keep.into(), &mut *tx).await?;
        prune_transactions(oldest_to_keep.into(), &mut *tx).await?;
        prune_pre_commits(oldest_to_keep.into(), &mut *tx).await?;
        prune_blocks(oldest_to_keep.into(), &mut *tx).await?;
        update_last_pruned(current_height.into(), &mut *tx).await?;

        let commit_start = Instant::now();
        tx.commit()
            .await
            .map_err(|source| ScraperError::StorageTxCommitFailure { source })?;
        log_db_operation_time("committing pruning tx", commit_start);

        log_db_operation_time("pruning storage", start);
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn begin_processing_tx(&self) -> Result<StorageTransaction, ScraperError> {
        debug!("starting storage tx");
        self.manager
            .connection_pool
            .begin()
            .await
            .map_err(|source| ScraperError::StorageTxBeginFailure { source })
    }

    pub async fn lowest_block_height(&self) -> Result<Option<i64>, ScraperError> {
        Ok(self.manager.get_lowest_block().await?)
    }

    pub async fn get_first_block_height_after(
        &self,
        time: OffsetDateTime,
    ) -> Result<Option<i64>, ScraperError> {
        Ok(self.manager.get_first_block_height_after(time).await?)
    }

    pub async fn get_last_block_height_before(
        &self,
        time: OffsetDateTime,
    ) -> Result<Option<i64>, ScraperError> {
        Ok(self.manager.get_last_block_height_before(time).await?)
    }

    pub async fn get_blocks_between(
        &self,
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
    ) -> Result<i64, ScraperError> {
        let Some(block_start) = self.get_first_block_height_after(start_time).await? else {
            return Ok(0);
        };
        let Some(block_end) = self.get_last_block_height_before(end_time).await? else {
            return Ok(0);
        };

        Ok(block_end - block_start)
    }

    pub async fn get_signed_between(
        &self,
        consensus_address: &str,
        start_height: i64,
        end_height: i64,
    ) -> Result<i32, ScraperError> {
        Ok(self
            .manager
            .get_signed_between(consensus_address, start_height, end_height)
            .await?)
    }

    pub async fn get_signed_between_times(
        &self,
        consensus_address: &str,
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
    ) -> Result<i32, ScraperError> {
        let Some(block_start) = self.get_first_block_height_after(start_time).await? else {
            return Ok(0);
        };
        let Some(block_end) = self.get_last_block_height_before(end_time).await? else {
            return Ok(0);
        };

        self.get_signed_between(consensus_address, block_start, block_end)
            .await
    }

    pub async fn get_precommit(
        &self,
        consensus_address: &str,
        height: i64,
    ) -> Result<Option<CommitSignature>, ScraperError> {
        Ok(self
            .manager
            .get_precommit(consensus_address, height)
            .await?)
    }

    pub async fn get_block_signers(&self, height: i64) -> Result<Vec<Validator>, ScraperError> {
        Ok(self.manager.get_block_validators(height).await?)
    }

    pub async fn get_all_known_validators(&self) -> Result<Vec<Validator>, ScraperError> {
        Ok(self.manager.get_validators().await?)
    }

    pub async fn get_last_processed_height(&self) -> Result<i64, ScraperError> {
        Ok(self.manager.get_last_processed_height().await?)
    }

    pub async fn get_pruned_height(&self) -> Result<i64, ScraperError> {
        Ok(self.manager.get_pruned_height().await?)
    }
}

pub async fn persist_block(
    block: &FullBlockInformation,
    tx: &mut StorageTransaction,
) -> Result<(), ScraperError> {
    let total_gas = crate::helpers::tx_gas_sum(&block.transactions);

    // SANITY CHECK: make sure the block proposer is present in the validator set
    block.ensure_proposer()?;

    // persist validators
    persist_validators(&block.validators, tx).await?;

    // persist block data
    persist_block_data(&block.block, total_gas, tx).await?;

    // persist commits
    if let Some(commit) = &block.block.last_commit {
        persist_commits(commit, &block.validators, tx).await?;
    } else {
        warn!("no commits for block {}", block.block.header.height)
    }

    // persist txs
    persist_txs(&block.transactions, tx).await?;

    // persist messages (inside the transactions)
    persist_messages(&block.transactions, tx).await?;

    update_last_processed(block.block.header.height.into(), tx.as_mut()).await?;

    Ok(())
}

async fn persist_validators(
    validators: &validators::Response,
    tx: &mut StorageTransaction,
) -> Result<(), ScraperError> {
    debug!("persisting {} validators", validators.total);
    for validator in &validators.validators {
        let consensus_address = crate::helpers::validator_consensus_address(validator.address)?;
        let consensus_pubkey = crate::helpers::validator_pubkey_to_bech32(validator.pub_key)?;

        insert_validator(
            consensus_address.to_string(),
            consensus_pubkey.to_string(),
            tx.as_mut(),
        )
        .await?;
    }

    Ok(())
}

async fn persist_block_data(
    block: &Block,
    total_gas: i64,
    tx: &mut StorageTransaction,
) -> Result<(), ScraperError> {
    let proposer_address =
        crate::helpers::validator_consensus_address(block.header.proposer_address)?.to_string();

    insert_block(
        block.header.height.into(),
        block.header.hash().to_string(),
        block.data.len() as u32,
        total_gas,
        proposer_address,
        block.header.time.into(),
        tx.as_mut(),
    )
    .await?;
    Ok(())
}

async fn persist_commits(
    commits: &Commit,
    validators: &validators::Response,
    tx: &mut StorageTransaction,
) -> Result<(), ScraperError> {
    debug!("persisting up to {} commits", commits.signatures.len());
    let height: i64 = commits.height.into();

    for commit_sig in &commits.signatures {
        let (validator_id, timestamp, signature) = match commit_sig {
            CommitSig::BlockIdFlagAbsent => {
                trace!("absent signature");
                continue;
            }
            CommitSig::BlockIdFlagCommit {
                validator_address,
                timestamp,
                signature,
            } => (validator_address, timestamp, signature),
            CommitSig::BlockIdFlagNil {
                validator_address,
                timestamp,
                signature,
            } => (validator_address, timestamp, signature),
        };

        let validator = crate::helpers::validator_info(*validator_id, validators)?;
        let validator_address = crate::helpers::validator_consensus_address(*validator_id)?;

        if signature.is_none() {
            warn!("empty signature for {validator_address} at height {height}");
            continue;
        }

        insert_precommit(
            validator_address.to_string(),
            height,
            (*timestamp).into(),
            validator.power.into(),
            validator.proposer_priority.value(),
            tx.as_mut(),
        )
        .await?;
    }

    Ok(())
}

async fn persist_txs(
    txs: &[ParsedTransactionResponse],
    tx: &mut StorageTransaction,
) -> Result<(), ScraperError> {
    debug!("persisting {} txs", txs.len());

    for chain_tx in txs {
        insert_transaction(
            chain_tx.hash.to_string(),
            chain_tx.height.into(),
            chain_tx.index as i64,
            chain_tx.tx_result.code.is_ok(),
            chain_tx.tx.body.messages.len() as i64,
            chain_tx.tx.body.memo.clone(),
            chain_tx.tx_result.gas_wanted,
            chain_tx.tx_result.gas_used,
            chain_tx.tx_result.log.clone(),
            tx.as_mut(),
        )
        .await?;
    }

    Ok(())
}

async fn persist_messages(
    txs: &[ParsedTransactionResponse],
    tx: &mut StorageTransaction,
) -> Result<(), ScraperError> {
    debug!("persisting messages");

    for chain_tx in txs {
        for (index, msg) in chain_tx.tx.body.messages.iter().enumerate() {
            insert_message(
                chain_tx.hash.to_string(),
                index as i64,
                msg.type_url.clone(),
                chain_tx.height.into(),
                tx.as_mut(),
            )
            .await?
        }
    }

    Ok(())
}
