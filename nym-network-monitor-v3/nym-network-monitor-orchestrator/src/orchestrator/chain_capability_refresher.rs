// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{NodeAwaitingCapabilityRefresh, NodeChainCapability};
use anyhow::anyhow;
use async_trait::async_trait;
use futures::StreamExt;
use nym_task::ShutdownToken;
use nym_validator_client::nyxd::module_traits::feegrant::query::FeegrantQueryClient;
use nym_validator_client::nyxd::{AccountId, Coin, TendermintRpcClientExt};
use rand::RngExt;
use std::num::NonZeroUsize;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// The on-chain lookups the capability sweep needs.
///
/// A narrow seam rather than the broad `FeegrantQueryClient` / `CosmWasmClient`, whose methods
/// bottom out in raw ABCI queries that a unit test cannot reasonably fake: this trait is small
/// enough to mock directly, so tests inject balances and feegrant answers without an RPC endpoint. It
/// is blanket-implemented for any real nyxd query client, which is what the orchestrator passes in.
#[async_trait]
pub(crate) trait NodeChainQuerier: Send + Sync {
    /// The account's balance in `denom`, zero if it holds none.
    async fn balance(&self, address: &AccountId, denom: &str) -> anyhow::Result<Coin>;

    /// Whether the account holds at least one feegrant allowance.
    async fn is_feegrant_grantee(&self, address: &AccountId) -> anyhow::Result<bool>;
}

#[async_trait]
impl<C> NodeChainQuerier for C
where
    C: FeegrantQueryClient + Send + Sync,
{
    async fn balance(&self, address: &AccountId, denom: &str) -> anyhow::Result<Coin> {
        // `get_balance` lives on the RPC client trait; `CosmWasmClient`'s bank methods are legacy compat
        Ok(
            TendermintRpcClientExt::get_balance(self, address, denom.to_string())
                .await?
                .unwrap_or_else(|| Coin::new(0, denom)),
        )
    }

    async fn is_feegrant_grantee(&self, address: &AccountId) -> anyhow::Result<bool> {
        // a coarse check, matching nym-api: the grant might be expired or not cover cosmwasm
        // execute messages, but its mere presence is enough for a first iteration
        Ok(!self
            .allowances(address.clone(), None)
            .await?
            .allowances
            .is_empty())
    }
}

/// How often the sweep loop wakes to pick up nodes that have become due or been newly bonded. Much
/// shorter than the per-node TTL: the TTL sets how often each node is re-queried, while this sets how
/// promptly a due node is noticed and bounds how long a newly bonded node waits for its first lookup.
const SWEEP_CHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// The value configuration the capability sweep needs, grouped so construction takes one bundle
/// rather than a long argument list.
pub(crate) struct ChainCapabilityConfig {
    /// Denom balances are queried in (the rewarding denom).
    pub(crate) denom: String,

    /// Base period after which a cached row is due to be re-queried, before jitter.
    pub(crate) ttl: Duration,

    /// Upper bound on the random jitter added to each node's next-due time, so a population cached
    /// together does not all fall due at once.
    pub(crate) jitter: Duration,

    /// Maximum number of nodes queried concurrently.
    pub(crate) concurrency: NonZeroUsize,
}

/// Keeps the `node_chain_capability` cache warm so config-score materialisation only ever READS it.
///
/// Wakes every [`SWEEP_CHECK_INTERVAL`], well away from the epoch transition, and (re)queries only
/// the nodes whose cached standing is missing or past its jittered next-due time, with bounded
/// concurrency. The raw balance is stored so the minimum-balance threshold is applied at score time
/// rather than baked in here.
pub(crate) struct ChainCapabilityRefresher<C> {
    querier: C,
    storage: NetworkMonitorStorage,
    config: ChainCapabilityConfig,
    shutdown_token: ShutdownToken,
}

impl<C: NodeChainQuerier> ChainCapabilityRefresher<C> {
    pub(crate) fn new(
        config: ChainCapabilityConfig,
        querier: C,
        storage: NetworkMonitorStorage,
        shutdown_token: ShutdownToken,
    ) -> Self {
        ChainCapabilityRefresher {
            querier,
            storage,
            config,
            shutdown_token,
        }
    }

    /// One sweep: query the due nodes and upsert whatever came back. Only successful lookups are
    /// written, so a node whose query failed keeps its last known value rather than being overwritten.
    async fn refresh(&self) -> anyhow::Result<()> {
        let due = self.storage.get_nodes_awaiting_capability_refresh().await?;
        if due.is_empty() {
            return Ok(());
        }
        debug!("refreshing chain capabilities for {} node(s)", due.len());

        let querier = &self.querier;
        let config = &self.config;
        let concurrency = self.config.concurrency.get();

        let capabilities: Vec<NodeChainCapability> = futures::stream::iter(due)
            .map(|node| async move {
                match query_node(config, querier, &node).await {
                    Ok(capability) => Some(capability),
                    Err(err) => {
                        debug!(
                            node_id = node.node_id,
                            "chain capability query failed: {err}"
                        );
                        None
                    }
                }
            })
            .buffer_unordered(concurrency)
            .filter_map(|capability| async move { capability })
            .collect()
            .await;

        if !capabilities.is_empty() {
            self.storage
                .batch_upsert_node_chain_capabilities(&capabilities)
                .await?;
        }
        Ok(())
    }

    /// Sweeps on startup (so a fresh cache fills without waiting a whole interval) and then every
    /// `refresh_interval`. A failed sweep is logged and left for the next one rather than killing the
    /// task, which would freeze every node's cached standing.
    pub(crate) async fn run(self) {
        loop {
            if let Err(err) = self.refresh().await {
                warn!("chain capability refresh cycle failed: {err}");
            }

            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => break,
                _ = sleep(SWEEP_CHECK_INTERVAL) => {}
            }
        }

        info!("chain capability refresher stopped");
    }
}

/// Queries one node's balance and feegrant status into a storable row. A feegrant blip is
/// non-fatal (the balance is kept and the node treated as not a grantee, matching nym-api), but a
/// balance failure skips the node so a stale value is left in place.
async fn query_node<C: NodeChainQuerier>(
    config: &ChainCapabilityConfig,
    querier: &C,
    node: &NodeAwaitingCapabilityRefresh,
) -> anyhow::Result<NodeChainCapability> {
    // validated when the node was described, so a failure here means corruption
    let address = node
        .declared_chain_address
        .parse::<AccountId>()
        .map_err(|err| {
            anyhow!(
                "stored on-chain address of node {} does not parse: {err}",
                node.node_id
            )
        })?;

    let balance = querier.balance(&address, &config.denom).await?;

    let is_feegrant_grantee = match querier.is_feegrant_grantee(&address).await {
        Ok(is_grantee) => is_grantee,
        Err(err) => {
            warn!(
                node_id = node.node_id,
                "failed to query feegrant, treating as not a grantee: {err}"
            );
            false
        }
    };

    let now = OffsetDateTime::now_utc();
    Ok(NodeChainCapability {
        node_id: node.node_id,
        balance: balance.to_string(),
        is_feegrant_grantee,
        refreshed_at: now,
        next_refresh_due_at: now + config.ttl + random_jitter(config.jitter),
    })
}

/// A random duration in `[0, jitter]`, added to each node's next-due time so that a population cached
/// together does not all fall due at the same instant.
fn random_jitter(jitter: Duration) -> Duration {
    // an inclusive `0..=0` is a valid, non-empty range that yields 0, so a zero jitter needs no guard
    Duration::from_secs(rand::rng().random_range(0..=jitter.as_secs()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{FIXTURE_SEEN_AT, mixnode};
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    const DENOM: &str = "unym";

    /// The on-chain address the fixture describes node `node_id` with, so a test can recognise
    /// which node the mock was asked about.
    fn address_of(node_id: i64) -> String {
        mixnode(node_id).description.unwrap().declared_chain_address
    }

    /// Storage holding described mixnodes `node_ids`, each with an on-chain address.
    async fn storage_with_nodes(node_ids: impl IntoIterator<Item = i64>) -> NetworkMonitorStorage {
        let storage = NetworkMonitorStorage::in_memory().await;
        let nodes: Vec<_> = node_ids.into_iter().map(mixnode).collect();
        storage
            .store_refresh(&nodes, FIXTURE_SEEN_AT)
            .await
            .unwrap();
        storage
    }

    async fn cached(storage: &NetworkMonitorStorage) -> Vec<NodeChainCapability> {
        storage
            .storage_manager
            .get_node_chain_capabilities()
            .await
            .unwrap()
    }

    /// A configurable mock of the chain lookups, recording which addresses it was asked about and the
    /// peak number of concurrent in-flight queries.
    struct MockQuerier {
        queried: Arc<Mutex<Vec<String>>>,
        in_flight: Arc<AtomicUsize>,
        max_in_flight: Arc<AtomicUsize>,
        delay: Duration,
        balance: u128,
        feegrant: bool,
    }

    fn mock() -> MockQuerier {
        MockQuerier {
            queried: Arc::new(Mutex::new(Vec::new())),
            in_flight: Arc::new(AtomicUsize::new(0)),
            max_in_flight: Arc::new(AtomicUsize::new(0)),
            delay: Duration::ZERO,
            balance: 0,
            feegrant: false,
        }
    }

    #[async_trait]
    impl NodeChainQuerier for MockQuerier {
        async fn balance(&self, address: &AccountId, denom: &str) -> anyhow::Result<Coin> {
            let current = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_in_flight.fetch_max(current, Ordering::SeqCst);
            self.queried.lock().unwrap().push(address.to_string());
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok(Coin::new(self.balance, denom))
        }

        async fn is_feegrant_grantee(&self, _address: &AccountId) -> anyhow::Result<bool> {
            Ok(self.feegrant)
        }
    }

    fn build_refresher(
        storage: NetworkMonitorStorage,
        querier: MockQuerier,
        ttl: Duration,
        jitter: Duration,
        concurrency: usize,
    ) -> ChainCapabilityRefresher<MockQuerier> {
        ChainCapabilityRefresher::new(
            ChainCapabilityConfig {
                denom: DENOM.to_string(),
                ttl,
                jitter,
                concurrency: NonZeroUsize::new(concurrency).unwrap(),
            },
            querier,
            storage,
            ShutdownToken::new(),
        )
    }

    #[tokio::test]
    async fn refresh_stores_the_raw_balance_and_feegrant() {
        let storage = storage_with_nodes([1]).await;

        let querier = MockQuerier {
            balance: 5000,
            feegrant: true,
            ..mock()
        };
        build_refresher(
            storage.clone(),
            querier,
            Duration::from_secs(3600),
            Duration::ZERO,
            4,
        )
        .refresh()
        .await
        .unwrap();

        let caps = cached(&storage).await;
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].node_id, 1);
        // the raw Coin is stored, not a sufficiency flag
        assert_eq!(caps[0].balance, Coin::new(5000, DENOM).to_string());
        assert!(caps[0].is_feegrant_grantee);
    }

    #[tokio::test]
    async fn only_due_nodes_are_requeried() {
        let storage = storage_with_nodes([1, 2]).await;

        // node 2 already has a cached entry whose next-due is comfortably in the future
        let now = OffsetDateTime::now_utc();
        storage
            .batch_upsert_node_chain_capabilities(&[NodeChainCapability {
                node_id: 2,
                balance: Coin::new(1, DENOM).to_string(),
                is_feegrant_grantee: false,
                refreshed_at: now,
                next_refresh_due_at: now + Duration::from_secs(3600),
            }])
            .await
            .unwrap();

        let queried = Arc::new(Mutex::new(Vec::new()));
        let querier = MockQuerier {
            queried: queried.clone(),
            balance: 100,
            ..mock()
        };
        build_refresher(
            storage.clone(),
            querier,
            Duration::from_secs(3600),
            Duration::ZERO,
            4,
        )
        .refresh()
        .await
        .unwrap();

        // only the node with no cached row (node 1) is queried; node 2 is not yet due
        assert_eq!(*queried.lock().unwrap(), vec![address_of(1)]);
    }

    #[tokio::test]
    async fn concurrency_is_bounded() {
        let storage = storage_with_nodes(1..=6).await;

        let max_in_flight = Arc::new(AtomicUsize::new(0));
        let querier = MockQuerier {
            max_in_flight: max_in_flight.clone(),
            // hold each query open long enough for the bound to actually bind
            delay: Duration::from_millis(20),
            ..mock()
        };
        let concurrency = 2;
        build_refresher(
            storage.clone(),
            querier,
            Duration::from_secs(3600),
            Duration::ZERO,
            concurrency,
        )
        .refresh()
        .await
        .unwrap();

        // more nodes than the limit, so the peak reaches - but never exceeds - the configured bound
        assert_eq!(max_in_flight.load(Ordering::SeqCst), concurrency);
    }

    #[tokio::test]
    async fn due_times_are_jittered_across_nodes() {
        let storage = storage_with_nodes(1..=10).await;

        let ttl = Duration::from_secs(24 * 3600);
        let jitter = Duration::from_secs(3600);
        let before = OffsetDateTime::now_utc();
        build_refresher(storage.clone(), mock(), ttl, jitter, 4)
            .refresh()
            .await
            .unwrap();
        let after = OffsetDateTime::now_utc();

        let caps = cached(&storage).await;
        assert_eq!(caps.len(), 10);

        // every next-due lands within [query_time + ttl, query_time + ttl + jitter]
        for cap in &caps {
            assert!(cap.next_refresh_due_at >= before + ttl);
            assert!(cap.next_refresh_due_at <= after + ttl + jitter);
        }

        // and they are spread rather than identical - a synchronised population would collapse to one
        let distinct: HashSet<_> = caps.iter().map(|c| c.next_refresh_due_at).collect();
        assert!(
            distinct.len() > 1,
            "expected jittered due times to differ, got {} distinct",
            distinct.len()
        );
    }
}
