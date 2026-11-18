// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::{
    BlockToProcess, DecodedMessage, FullBlockInformation, ParsedTransactionResponse,
};
use crate::error::ScraperError;
use crate::helpers::tx_hash;
use crate::{Any, MessageRegistry, ParsedTransactionDetails, default_message_registry};
use futures::future::{BoxFuture, join};
use std::collections::BTreeMap;
use std::future::Future;
use tendermint::Block;
use tendermint::abci::types::ExecTxResult;
use tendermint_rpc::endpoint::{block, block_results, validators};
use tendermint_rpc::{Client, HttpClient, Paging};
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

        // only wait if another attempt follows
        if i + 1 < max_attempts {
            tokio::time::sleep(std::time::Duration::from_millis(300 * (i as u64 + 1))).await;
        }
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
struct RpcEndpoint {
    url: Url,

    // right now I don't care about anything nym specific, so a simple http client is sufficient,
    // once this is inadequate, we can switch to a NyxdClient
    client: HttpClient,
}

#[derive(Clone)]
pub struct RpcClient {
    /// In order of preference: a query moves on to the next endpoint only once the previous one
    /// has failed it. Never empty.
    endpoints: Vec<RpcEndpoint>,

    // kinda like very limited cosmos sdk codec
    pub(crate) message_registry: MessageRegistry,
}

impl RpcClient {
    pub fn new(urls: &[Url]) -> Result<Self, ScraperError> {
        if urls.is_empty() {
            return Err(ScraperError::NoRpcEndpoints);
        }

        let mut endpoints = Vec::with_capacity(urls.len());
        for url in urls {
            let client = HttpClient::new(url.as_str()).map_err(|source| {
                ScraperError::HttpConnectionFailure {
                    url: url.to_string(),
                    source: Box::new(source),
                }
            })?;
            endpoints.push(RpcEndpoint {
                url: url.clone(),
                client,
            });
        }

        Ok(RpcClient {
            endpoints,
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

    /// Pair each tx of the block with its execution result, which come in the same order.
    fn parse_transactions(
        &self,
        block: &Block,
        tx_results: Vec<ExecTxResult>,
    ) -> Result<Vec<ParsedTransactionResponse>, ScraperError> {
        if tx_results.len() != block.data.len() {
            return Err(ScraperError::TxResultsCountMismatch {
                height: block.header.height.value() as u32,
                transactions: block.data.len(),
                results: tx_results.len(),
            });
        }

        let mut transactions = Vec::with_capacity(tx_results.len());
        for (index, (raw_tx, tx_result)) in block.data.iter().zip(tx_results).enumerate() {
            let hash = tx_hash(raw_tx);
            let tx = cosmrs::Tx::from_bytes(raw_tx)
                .map_err(|source| ScraperError::TxParseFailure { hash, source })?;

            let mut decoded_messages = BTreeMap::new();
            for (msg_index, msg) in tx.body.messages.iter().enumerate() {
                if let Some(decoded_content) = self.decode_or_skip(msg) {
                    decoded_messages.insert(
                        msg_index,
                        DecodedMessage {
                            type_url: msg.type_url.clone(),
                            decoded_content,
                        },
                    );
                }
            }

            transactions.push(ParsedTransactionResponse {
                tx_details: ParsedTransactionDetails {
                    hash,
                    index: index as u32,
                    tx_result,
                    tx,
                    proof: None,
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

        // tx results a block did not arrive with are all in its block results: one query, not one per tx
        let fetch_block_results =
            config.get_block_results || (config.get_transactions && block.lacks_tx_results());

        // make all the http requests run concurrently
        let (results, validators) = join(
            self.maybe_get_block_results(height, fetch_block_results),
            self.maybe_get_validators_details(height, config.get_validators),
        )
        .await;
        let results = results?;

        let tx_results = block
            .tx_results
            .or_else(|| results.as_ref()?.txs_results.clone())
            .unwrap_or_default();

        let transactions = if config.get_transactions {
            Some(self.parse_transactions(&block.block, tx_results)?)
        } else {
            None
        };

        Ok(FullBlockInformation {
            block: block.block,
            results: results.filter(|_| config.get_block_results),
            validators: validators?,
            transactions,
        })
    }

    /// The first answer to `query` in endpoint order, or the last endpoint's failure.
    async fn first_answer<T>(
        &self,
        query: impl for<'a> Fn(&'a HttpClient) -> BoxFuture<'a, Result<T, tendermint_rpc::Error>>,
    ) -> Result<T, tendermint_rpc::Error> {
        let mut failure = None;
        for endpoint in &self.endpoints {
            match query(&endpoint.client).await {
                Ok(answer) => return Ok(answer),
                Err(err) => {
                    debug!("{} failed the query: {err}", endpoint.url);
                    failure = Some(err);
                }
            }
        }

        // SAFETY: the client is never built without an endpoint, so at least one query has failed
        #[allow(clippy::unwrap_used)]
        Err(failure.unwrap())
    }

    #[instrument(skip(self))]
    pub async fn get_basic_block_details(
        &self,
        height: u32,
    ) -> Result<block::Response, ScraperError> {
        debug!("getting basic block details");

        self.first_answer(|client| client.block(height))
            .await
            .map_err(|source| ScraperError::RpcQueryFailure {
                query: format!("basic block information at height {height}"),
                source: Box::new(source),
            })
    }

    #[instrument(skip(self))]
    pub async fn get_block_results(
        &self,
        height: u32,
    ) -> Result<block_results::Response, ScraperError> {
        debug!("getting block results");

        self.first_answer(|client| client.block_results(height))
            .await
            .map_err(|source| ScraperError::RpcQueryFailure {
                query: format!("block results at height {height}"),
                source: Box::new(source),
            })
    }

    #[instrument(skip(self))]
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

        let info = self
            .first_answer(|client| client.abci_info())
            .await
            .map_err(|source| ScraperError::RpcQueryFailure {
                query: "current abci info".to_string(),
                source: Box::new(source),
            })?;
        Ok(info.last_block_height.value())
    }

    pub(crate) async fn earliest_available_block_height(&self) -> Result<u64, ScraperError> {
        debug!("getting earliest available block height");

        let status = self
            .first_answer(|client| client.status())
            .await
            .map_err(|source| ScraperError::RpcQueryFailure {
                query: "node status".to_string(),
                source: Box::new(source),
            })?;
        Ok(status.sync_info.earliest_block_height.value())
    }

    #[instrument(skip(self))]
    pub async fn get_validators_details(
        &self,
        height: u32,
    ) -> Result<validators::Response, ScraperError> {
        debug!("getting validators set");

        self.first_answer(|client| client.validators(height, Paging::All))
            .await
            .map_err(|source| ScraperError::RpcQueryFailure {
                query: format!("validators at height {height}"),
                source: Box::new(source),
            })
    }

    #[instrument(skip(self))]
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

#[cfg(test)]
mod tests {
    use super::test_helpers::client_without_a_node;
    use super::*;
    use crate::block_processor::types::test_helpers::{
        NEW_BLOCK_MESSAGE, TX_GAS_USED, TX_HASH, parse_new_block_message,
    };
    use futures::FutureExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    const TRANSACTIONS_ONLY: RetrievalConfig = RetrievalConfig {
        get_validators: false,
        get_transactions: true,
        get_block_results: false,
    };

    // With no endpoint there is nothing to query. That has to surface when the client is built,
    // not as a failure of the first block that needs fetching.
    #[test]
    fn a_client_is_not_built_without_an_endpoint() {
        assert!(matches!(
            RpcClient::new(&[]),
            Err(ScraperError::NoRpcEndpoints)
        ));
    }

    fn recorded_block() -> BlockToProcess {
        BlockToProcess::try_from(parse_new_block_message(NEW_BLOCK_MESSAGE))
            .expect("the recorded event must convert")
    }

    // A live block carries everything its transactions need. Looking any of it up again brings
    // back the race against the node's tx indexer, and a query from every node for every tx.
    #[tokio::test]
    async fn a_block_that_arrived_with_its_tx_results_needs_no_rpc_query() {
        let details = client_without_a_node()
            .try_get_full_details(recorded_block(), TRANSACTIONS_ONLY)
            .await
            .expect("an rpc query was attempted");

        let transactions = details.transactions.expect("transactions were asked for");
        assert_eq!(1, transactions.len());

        let tx = &transactions[0].tx_details;
        assert_eq!(TX_HASH, tx.hash.to_string());
        assert_eq!(0, tx.index);
        assert_eq!(TX_GAS_USED, tx.tx_result.gas_used);
    }

    // Txs paired against too few results would silently vanish from the block. It has to be an
    // error instead, so that the block is retried.
    #[test]
    fn a_block_is_not_paired_with_fewer_results_than_it_has_txs() {
        let block = recorded_block();

        let res = client_without_a_node().parse_transactions(&block.block, Vec::new());

        assert!(matches!(
            res,
            Err(ScraperError::TxResultsCountMismatch {
                transactions: 1,
                results: 0,
                ..
            })
        ));
    }

    // A fallback is only there for when the endpoints before it fail, and once one has answered
    // there is nothing left to ask the rest.
    #[tokio::test]
    async fn endpoints_are_asked_in_turn_until_one_answers() {
        let url: Url = "http://127.0.0.1:1".parse().expect("the url is valid");
        let client = RpcClient::new(&[url.clone(), url.clone(), url])
            .expect("building the client opens no connection");

        let asked = AtomicUsize::new(0);
        let answer = client
            .first_answer(|_| {
                let call = asked.fetch_add(1, Ordering::Relaxed);
                async move {
                    if call == 0 {
                        Err(tendermint_rpc::Error::client_internal(
                            "the first endpoint fails".to_string(),
                        ))
                    } else {
                        Ok(call)
                    }
                }
                .boxed()
            })
            .await
            .expect("the second endpoint answered");

        assert_eq!(1, answer);
        assert_eq!(2, asked.load(Ordering::Relaxed));
    }

    // A failed query is reported once its last attempt has failed: there is nothing left to wait for.
    #[tokio::test(start_paused = true)]
    async fn the_last_failed_attempt_is_not_followed_by_a_wait() {
        let started = tokio::time::Instant::now();

        let res: Result<(), ScraperError> =
            query_with_retries(3, || async { Err(ScraperError::NoModulesConfigured) }).await;

        assert!(res.is_err());
        // the waits between three attempts: 300ms, then 600ms
        assert_eq!(Duration::from_millis(900), started.elapsed());
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::RpcClient;

    // nothing listens on this port, so a call that attempts any rpc query fails
    pub(crate) fn client_without_a_node() -> RpcClient {
        let url = "http://127.0.0.1:1".parse().expect("the url is valid");
        RpcClient::new(&[url]).expect("building the client opens no connection")
    }
}
