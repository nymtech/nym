// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NyxdScraperStorageError;
use crate::{NyxdScraperStorage, NyxdScraperTransaction, ParsedTransactionResponse};
use tendermint::Block;
use tendermint::block::Commit;
use tendermint_rpc::endpoint::validators::Response;
use thiserror::Error;

#[derive(Clone)]
pub struct Ephemeral;

#[derive(Debug, Error)]
#[error("no storage backend enabled")]
pub struct EphemeralStorageError;

pub struct EphemeralTransaction;

#[async_trait::async_trait]
impl NyxdScraperTransaction for EphemeralTransaction {
    async fn commit(self) -> Result<(), NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn persist_validators(&mut self, _: &Response) -> Result<(), NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn persist_block_data(
        &mut self,
        _: &Block,
        _: i64,
    ) -> Result<(), NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn persist_commits(
        &mut self,
        _: &Commit,
        _: &Response,
    ) -> Result<(), NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn persist_txs(
        &mut self,
        _: &[ParsedTransactionResponse],
    ) -> Result<(), NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn persist_messages(
        &mut self,
        _: &[ParsedTransactionResponse],
    ) -> Result<(), NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn update_last_processed(&mut self, _: i64) -> Result<(), NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }
}

#[async_trait::async_trait]
impl NyxdScraperStorage for Ephemeral {
    type StorageTransaction = EphemeralTransaction;

    async fn initialise(_: &str, _: &bool) -> Result<Self, NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn begin_processing_tx(
        &self,
    ) -> Result<Self::StorageTransaction, NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn get_last_processed_height(&self) -> Result<i64, NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn get_pruned_height(&self) -> Result<i64, NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn lowest_block_height(&self) -> Result<Option<i64>, NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }

    async fn prune_storage(&self, _: u32, _: u32) -> Result<(), NyxdScraperStorageError> {
        Err(NyxdScraperStorageError::new(EphemeralStorageError))
    }
}
