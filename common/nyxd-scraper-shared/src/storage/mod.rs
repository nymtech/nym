// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ScraperError;
use async_trait::async_trait;
use std::path::Path;
use thiserror::Error;
use tracing::warn;

pub use crate::block_processor::types::FullBlockInformation;
pub use crate::ParsedTransactionResponse;
pub use tendermint::block::{Commit, CommitSig};
pub use tendermint::Block;
pub use tendermint_rpc::endpoint::validators;

pub mod helpers;

// a workaround for needing associated type (which is a no-no in dynamic dispatch)
#[derive(Error, Debug)]
#[error(transparent)]
pub struct NyxdScraperStorageError(Box<dyn std::error::Error + Send + Sync>);

impl NyxdScraperStorageError {
    pub fn new<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        NyxdScraperStorageError(Box::new(error))
    }
}

#[async_trait]
pub trait NyxdScraperStorage: Clone + Sized {
    type StorageTransaction: NyxdScraperTransaction;

    async fn initialise(storage_path: &Path) -> Result<Self, NyxdScraperStorageError>;

    async fn begin_processing_tx(
        &self,
    ) -> Result<Self::StorageTransaction, NyxdScraperStorageError>;

    async fn get_last_processed_height(&self) -> Result<i64, NyxdScraperStorageError>;

    async fn get_pruned_height(&self) -> Result<i64, NyxdScraperStorageError>;

    async fn lowest_block_height(&self) -> Result<Option<i64>, NyxdScraperStorageError>;

    async fn prune_storage(
        &self,
        oldest_to_keep: u32,
        current_height: u32,
    ) -> Result<(), NyxdScraperStorageError>;
}

#[async_trait]
pub trait NyxdScraperTransaction {
    async fn commit(mut self) -> Result<(), NyxdScraperStorageError>;

    async fn persist_validators(
        &mut self,
        validators: &validators::Response,
    ) -> Result<(), NyxdScraperStorageError>;

    async fn persist_block_data(
        &mut self,
        block: &Block,
        total_gas: i64,
    ) -> Result<(), NyxdScraperStorageError>;

    async fn persist_commits(
        &mut self,
        commits: &Commit,
        validators: &validators::Response,
    ) -> Result<(), NyxdScraperStorageError>;

    async fn persist_txs(
        &mut self,
        txs: &[ParsedTransactionResponse],
    ) -> Result<(), NyxdScraperStorageError>;

    async fn persist_messages(
        &mut self,
        txs: &[ParsedTransactionResponse],
    ) -> Result<(), NyxdScraperStorageError>;

    async fn update_last_processed(&mut self, height: i64) -> Result<(), NyxdScraperStorageError>;
}

pub async fn persist_block<Tx>(
    block: &FullBlockInformation,
    tx: &mut Tx,
    store_precommits: bool,
) -> Result<(), ScraperError>
where
    Tx: NyxdScraperTransaction,
{
    let total_gas = crate::helpers::tx_gas_sum(&block.transactions);

    // SANITY CHECK: make sure the block proposer is present in the validator set
    block.ensure_proposer()?;

    tx.persist_validators(&block.validators).await?;

    tx.persist_block_data(&block.block, total_gas).await?;

    if store_precommits {
        if let Some(commit) = &block.block.last_commit {
            tx.persist_commits(commit, &block.validators).await?;
        } else {
            warn!("no commits for block {}", block.block.header.height)
        }
    }

    // persist txs
    tx.persist_txs(&block.transactions).await?;

    // persist messages (inside the transactions)
    tx.persist_messages(&block.transactions).await?;

    tx.update_last_processed(block.block.header.height.into())
        .await?;

    Ok(())
}
