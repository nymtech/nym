// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::{
    BlockToProcess, FullBlockInformation, ParsedTransactionResponse,
};
use crate::error::ScraperError;
use crate::helpers::tx_hash;
use futures::future::join3;
use futures::StreamExt;
use std::collections::BTreeMap;
use std::sync::Arc;
use tendermint::Hash;
use tendermint_rpc::endpoint::{block, block_results, tx, validators};
use tendermint_rpc::{Client, HttpClient, Paging};
use tokio::sync::Mutex;
use tracing::{debug, instrument};
use url::Url;

#[derive(Clone)]
pub struct RpcClient {
    // right now I don't care about anything nym specific, so a simple http client is sufficient,
    // once this is inadequate, we can switch to a NyxdClient
    inner: Arc<HttpClient>,
}

impl RpcClient {
    pub fn new(url: &Url) -> Result<Self, ScraperError> {
        let http_client = HttpClient::new(url.as_str()).map_err(|source| {
            ScraperError::HttpConnectionFailure {
                url: url.to_string(),
                source,
            }
        })?;

        Ok(RpcClient {
            inner: Arc::new(http_client),
        })
    }

    #[instrument(skip(self, block), fields(height = block.height))]
    pub async fn try_get_full_details(
        &self,
        block: BlockToProcess,
    ) -> Result<FullBlockInformation, ScraperError> {
        debug!("getting complete block details");
        let height = block.height;

        // make all the http requests concurrently
        let (results, validators, raw_transactions) = join3(
            self.get_block_results(height),
            self.get_validators_details(height),
            self.get_transaction_results(&block.block.data),
        )
        .await;

        let raw_transactions = raw_transactions?;
        let mut transactions = Vec::with_capacity(raw_transactions.len());
        for tx in raw_transactions {
            transactions.push(ParsedTransactionResponse {
                hash: tx.hash,
                height: tx.height,
                index: tx.index,
                tx_result: tx.tx_result,
                tx: cosmrs::Tx::from_bytes(&tx.tx).map_err(|source| {
                    ScraperError::TxParseFailure {
                        hash: tx.hash,
                        source,
                    }
                })?,
                proof: tx.proof,
            })
        }

        Ok(FullBlockInformation {
            block: block.block,
            results: results?,
            validators: validators?,
            transactions,
        })
    }

    #[instrument(skip(self), err(Display))]
    pub async fn get_basic_block_details(
        &self,
        height: u32,
    ) -> Result<block::Response, ScraperError> {
        debug!("getting basic block details");

        self.inner
            .block(height)
            .await
            .map_err(|source| ScraperError::BlockQueryFailure { height, source })
    }

    #[instrument(skip(self), err(Display))]
    pub async fn get_block_results(
        &self,
        height: u32,
    ) -> Result<block_results::Response, ScraperError> {
        debug!("getting block results");

        self.inner
            .block_results(height)
            .await
            .map_err(|source| ScraperError::BlockResultsQueryFailure { height, source })
    }

    pub(crate) async fn current_block_height(&self) -> Result<u64, ScraperError> {
        debug!("getting current block height");

        let info = self
            .inner
            .abci_info()
            .await
            .map_err(|source| ScraperError::AbciInfoQueryFailure { source })?;
        Ok(info.last_block_height.value())
    }

    async fn get_transaction_results(
        &self,
        raw: &[Vec<u8>],
    ) -> Result<Vec<tx::Response>, ScraperError> {
        let ordered_results = Arc::new(Mutex::new(BTreeMap::new()));

        // "Data is just a wrapper for a list of transactions, where transactions are arbitrary byte arrays"
        // source: https://github.com/tendermint/spec/blob/d46cd7f573a2c6a2399fcab2cde981330aa63f37/spec/core/data_structures.md#data
        //
        // I hate that zip as much as you, dear reader, but for some reason the compiler didn't let me remove the `move`
        futures::stream::iter(
            raw.iter()
                .map(tx_hash)
                .enumerate()
                .zip(std::iter::repeat(ordered_results.clone())),
        )
        .for_each_concurrent(4, |((id, tx_hash), ordered_results)| async move {
            let res = self.get_transaction_result(tx_hash).await;
            ordered_results.lock().await.insert(id, res);
        })
        .await;

        // safety the futures have completed so we MUST have the only arc reference
        #[allow(clippy::unwrap_used)]
        let inner = Arc::into_inner(ordered_results).unwrap().into_inner();

        // BTreeMap is ordered by its keys so we're guaranteed to get txs in correct order
        inner.into_values().collect()
    }

    #[instrument(skip(self, tx_hash), fields(tx_hash = %tx_hash), err(Display))]
    async fn get_transaction_result(&self, tx_hash: Hash) -> Result<tx::Response, ScraperError> {
        debug!("getting tx results for {tx_hash}");

        self.inner
            .tx(tx_hash, false)
            .await
            .map_err(|source| ScraperError::TxResultsQueryFailure {
                hash: tx_hash,
                source,
            })
    }

    #[instrument(skip(self))]
    pub async fn get_validators_details(
        &self,
        height: u32,
    ) -> Result<validators::Response, ScraperError> {
        debug!("getting validators set");

        self.inner
            .validators(height, Paging::All)
            .await
            .map_err(|source| ScraperError::ValidatorsQueryFailure { height, source })
    }
}
