// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::FullBlockInformation;
use crate::error::ScraperError;
use crate::ParsedTransactionResponse;
use async_trait::async_trait;
use std::path::PathBuf;
use tendermint::block::Commit;
use tendermint::Block;
use tendermint_rpc::endpoint::validators;
use tracing::warn;

pub(crate) mod helpers;

#[async_trait]
pub trait NyxdScraperStorage: Clone + Sized {
    type Error: Into<ScraperError> + std::error::Error + Send + Sync;
    type StorageTransaction: NyxdScraperTransaction<Error = Self::Error>;

    async fn initialise(storage_path: &PathBuf) -> Result<Self, Self::Error>;

    async fn begin_processing_tx(&self) -> Result<Self::StorageTransaction, Self::Error>;

    async fn get_last_processed_height(&self) -> Result<u32, Self::Error>;

    async fn get_pruned_height(&self) -> Result<u32, Self::Error>;

    async fn lowest_block_height(&self) -> Result<Option<i64>, Self::Error>;

    async fn prune_storage(
        &self,
        oldest_to_keep: u32,
        current_height: u32,
    ) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait NyxdScraperTransaction {
    type Error: Into<ScraperError> + std::error::Error + Send + Sync;

    async fn commit(mut self) -> Result<(), Self::Error>;

    async fn persist_validators(
        &mut self,
        validators: &validators::Response,
    ) -> Result<(), Self::Error>;

    async fn persist_block_data(
        &mut self,
        block: &Block,
        total_gas: i64,
    ) -> Result<(), Self::Error>;

    async fn persist_commits(
        &mut self,
        commits: &Commit,
        validators: &validators::Response,
    ) -> Result<(), Self::Error>;

    async fn persist_txs(&mut self, txs: &[ParsedTransactionResponse]) -> Result<(), Self::Error>;

    async fn persist_messages(
        &mut self,
        txs: &[ParsedTransactionResponse],
    ) -> Result<(), Self::Error>;

    async fn update_last_processed(&mut self, height: i64) -> Result<(), Self::Error>;
}

pub async fn persist_block<Tx>(
    block: &FullBlockInformation,
    tx: &mut Tx,
    store_precommits: bool,
) -> Result<(), ScraperError>
where
    Tx: NyxdScraperTransaction,
    ScraperError: From<<Tx as NyxdScraperTransaction>::Error>,
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
