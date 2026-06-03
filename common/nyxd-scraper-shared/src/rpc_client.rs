// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::{
    BlockToProcess, DecodedMessage, FullBlockInformation, ParsedTransactionResponse,
};
use crate::error::ScraperError;
use crate::helpers::tx_hash;
use crate::{Any, MessageRegistry, ParsedTransactionDetails, default_message_registry};
use futures::StreamExt;
use futures::future::join3;
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use tendermint::{Block, Hash};
use tendermint_rpc::endpoint::{block, block_results, tx, validators};
use tendermint_rpc::{Client, HttpClient, Paging};
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};
use url::Url;

const MAX_QUERY_ATTEMPTS: usize = 3;

/// Runs `op` up to `max_attempts` times (at least once), returning the first success or, on full
/// exhaustion, the last error encountered.
async fn query_with_retries<F, Fut, T>(mut max_attempts: usize, op: F) -> Result<T, ScraperError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, ScraperError>>,
{
    if max_attempts == 0 {
        max_attempts = 1;
    }

    let mut last_err = None;

    for i in 0..max_attempts {
        match op().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                debug!("query failed, retrying {}/{max_attempts} - {err}", i + 1);
                last_err = Some(err);
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(300 * (i as u64 + 1))).await;
    }

    // SAFETY: max_attempts >= 1, so we only reach here after at least one recorded failure
    #[allow(clippy::unwrap_used)]
    Err(last_err.unwrap())
}

#[derive(Debug, Clone, Copy)]
pub struct RetrievalConfig {
    pub get_validators: bool,
    pub get_transactions: bool,
    pub get_block_results: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            get_validators: true,
            get_transactions: true,
            get_block_results: true,
        }
    }
}

#[derive(Clone)]
pub struct RpcClient {
    // right now I don't care about anything nym specific, so a simple http client is sufficient,
    // once this is inadequate, we can switch to a NyxdClient
    inner: Arc<HttpClient>,

    // kinda like very limited cosmos sdk codec
    pub(crate) message_registry: MessageRegistry,
}

impl RpcClient {
    pub fn new(url: &Url) -> Result<Self, ScraperError> {
        let http_client = HttpClient::new(url.as_str()).map_err(|source| {
            ScraperError::HttpConnectionFailure {
                url: url.to_string(),
                source: Box::new(source),
            }
        })?;

        Ok(RpcClient {
            inner: Arc::new(http_client),
            message_registry: default_message_registry(),
        })
    }

    fn decode_or_skip(&self, msg: &Any) -> Option<serde_json::Value> {
        match self.message_registry.try_decode(msg) {
            Ok(decoded) => Some(decoded),
            Err(err) => {
                warn!("Failed to decode raw message: {err}");
                None
            }
        }
    }

    fn parse_transactions(
        &self,
        raw_transactions: Vec<tx::Response>,
        block: &Block,
    ) -> Result<Vec<ParsedTransactionResponse>, ScraperError> {
        let mut transactions = Vec::with_capacity(raw_transactions.len());
        for raw_tx in raw_transactions {
            let mut decoded_messages = BTreeMap::new();
            let tx = cosmrs::Tx::from_bytes(&raw_tx.tx).map_err(|source| {
                ScraperError::TxParseFailure {
                    hash: raw_tx.hash,
                    source,
                }
            })?;

            for (index, msg) in tx.body.messages.iter().enumerate() {
                if let Some(decoded_content) = self.decode_or_skip(msg) {
                    decoded_messages.insert(
                        index,
                        DecodedMessage {
                            type_url: msg.type_url.clone(),
                            decoded_content,
                        },
                    );
                }
            }

            transactions.push(ParsedTransactionResponse {
                tx_details: ParsedTransactionDetails {
                    hash: raw_tx.hash,
                    index: raw_tx.index,
                    tx_result: raw_tx.tx_result,
                    tx,
                    proof: raw_tx.proof,
                    block: block.clone(),
                },
                decoded_messages,
            })
        }
        Ok(transactions)
    }

    #[instrument(skip(self, block), fields(height = block.height))]
    pub async fn try_get_full_details(
        &self,
        block: BlockToProcess,
        config: RetrievalConfig,
    ) -> Result<FullBlockInformation, ScraperError> {
        debug!("getting complete block details");
        let height = block.height;

        // make all the http requests run concurrently
        let (results, validators, raw_transactions) = join3(
            self.maybe_get_block_results(height, config.get_block_results),
            self.maybe_get_validators_details(height, config.get_validators),
            self.maybe_get_transaction_results(&block.block.data, config.get_transactions),
        )
        .await;

        let transactions = match raw_transactions? {
            Some(raw) => Some(self.parse_transactions(raw, &block.block)?),
            None => None,
        };

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
            .map_err(|source| ScraperError::BlockQueryFailure {
                height,
                source: Box::new(source),
            })
    }

    #[instrument(skip(self), err(Display))]
    pub async fn get_block_results(
        &self,
        height: u32,
    ) -> Result<block_results::Response, ScraperError> {
        debug!("getting block results");

        self.inner.block_results(height).await.map_err(|source| {
            ScraperError::BlockResultsQueryFailure {
                height,
                source: Box::new(source),
            }
        })
    }

    #[instrument(skip(self), err(Display))]
    async fn get_block_results_with_retries(
        &self,
        height: u32,
        max_attempts: usize,
    ) -> Result<block_results::Response, ScraperError> {
        query_with_retries(max_attempts, || self.get_block_results(height)).await
    }

    async fn maybe_get_block_results(
        &self,
        height: u32,
        retrieve: bool,
    ) -> Result<Option<block_results::Response>, ScraperError> {
        if retrieve {
            self.get_block_results_with_retries(height, MAX_QUERY_ATTEMPTS)
                .await
                .map(Some)
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn current_block_height(&self) -> Result<u64, ScraperError> {
        debug!("getting current block height");

        let info =
            self.inner
                .abci_info()
                .await
                .map_err(|source| ScraperError::AbciInfoQueryFailure {
                    source: Box::new(source),
                })?;
        Ok(info.last_block_height.value())
    }

    pub(crate) async fn earliest_available_block_height(&self) -> Result<u64, ScraperError> {
        debug!("getting earliest available block height");

        let status =
            self.inner
                .status()
                .await
                .map_err(|source| ScraperError::AbciInfoQueryFailure {
                    source: Box::new(source),
                })?;
        Ok(status.sync_info.earliest_block_height.value())
    }

    async fn get_transaction_results(
        &self,
        raw: &[Vec<u8>],
    ) -> Result<Vec<tx::Response>, ScraperError> {
        let ordered_results = Arc::new(Mutex::new(BTreeMap::new()));

        // "Data is just a wrapper for a list of transactions, where transactions are arbitrary byte arrays"
        // source: https://github.com/tendermint/spec/blob/d46cd7f573a2c6a2399fcab2cde981330aa63f37/spec/core/data_structures.md#data
        futures::stream::iter(
            raw.iter()
                .map(tx_hash)
                .enumerate()
                .zip(std::iter::repeat(ordered_results.clone())),
        )
        .for_each_concurrent(4, |((id, tx_hash), ordered_results)| async move {
            let res = self
                .get_transaction_result_with_retries(tx_hash, MAX_QUERY_ATTEMPTS)
                .await;
            ordered_results.lock().await.insert(id, res);
        })
        .await;

        // safety: the futures have completed so we MUST have the only arc reference
        #[allow(clippy::unwrap_used)]
        let inner = Arc::into_inner(ordered_results).unwrap().into_inner();

        // BTreeMap is ordered by its keys so we're guaranteed to get txs in correct order
        inner.into_values().collect()
    }

    async fn maybe_get_transaction_results(
        &self,
        raw: &[Vec<u8>],
        retrieve: bool,
    ) -> Result<Option<Vec<tx::Response>>, ScraperError> {
        if retrieve {
            self.get_transaction_results(raw).await.map(Some)
        } else {
            Ok(None)
        }
    }

    #[instrument(skip(self, tx_hash), fields(tx_hash = %tx_hash), err(Display))]
    async fn get_transaction_result(&self, tx_hash: Hash) -> Result<tx::Response, ScraperError> {
        debug!("getting tx results");

        self.inner
            .tx(tx_hash, false)
            .await
            .map_err(|source| ScraperError::TxResultsQueryFailure {
                hash: tx_hash,
                source: Box::new(source),
            })
    }

    #[instrument(skip(self, tx_hash), fields(tx_hash = %tx_hash), err(Display))]
    async fn get_transaction_result_with_retries(
        &self,
        tx_hash: Hash,
        max_attempts: usize,
    ) -> Result<tx::Response, ScraperError> {
        query_with_retries(max_attempts, || self.get_transaction_result(tx_hash)).await
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
            .map_err(|source| ScraperError::ValidatorsQueryFailure {
                height,
                source: Box::new(source),
            })
    }

    #[instrument(skip(self), err(Display))]
    async fn get_validators_details_with_retries(
        &self,
        height: u32,
        max_attempts: usize,
    ) -> Result<validators::Response, ScraperError> {
        query_with_retries(max_attempts, || self.get_validators_details(height)).await
    }

    async fn maybe_get_validators_details(
        &self,
        height: u32,
        retrieve: bool,
    ) -> Result<Option<validators::Response>, ScraperError> {
        if retrieve {
            self.get_validators_details_with_retries(height, MAX_QUERY_ATTEMPTS)
                .await
                .map(Some)
        } else {
            Ok(None)
        }
    }
}
