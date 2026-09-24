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
use rand::Rng;
use std::str::FromStr;
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

/// Keeps the `node_chain_capability` cache warm so config-score materialisation only ever READS it.
///
/// Wakes every [`SWEEP_CHECK_INTERVAL`], well away from the epoch transition, and (re)queries only
/// the nodes whose cached standing is missing or past its jittered next-due time, with bounded
/// concurrency. The raw balance is stored so the minimum-balance threshold is applied at score time
/// rather than baked in here.
pub(crate) struct ChainCapabilityRefresher<C> {
    querier: C,
    storage: NetworkMonitorStorage,

    /// Denom balances are queried in (the rewarding denom).
    denom: String,

    /// Base period after which a cached row is due to be re-queried, before jitter.
    ttl: Duration,

    /// Upper bound on the random jitter added to each node's next-due time, so a population cached
    /// together does not all fall due at once.
    jitter: Duration,

    /// Maximum number of nodes queried concurrently.
    concurrency: usize,

    shutdown_token: ShutdownToken,
}

impl<C: NodeChainQuerier> ChainCapabilityRefresher<C> {
    pub(crate) fn new(
        querier: C,
        storage: NetworkMonitorStorage,
        denom: String,
        ttl: Duration,
        jitter: Duration,
        concurrency: usize,
        shutdown_token: ShutdownToken,
    ) -> Self {
        ChainCapabilityRefresher {
            querier,
            storage,
            denom,
            ttl,
            jitter,
            concurrency,
            shutdown_token,
        }
    }

    /// One sweep: query the due nodes and upsert whatever came back. Only successful lookups are
    /// written, so a node whose query failed keeps its last known value rather than being overwritten.
    async fn refresh(&self) -> anyhow::Result<()> {
        let now = OffsetDateTime::now_utc();
        let due = self.storage.nodes_awaiting_capability_refresh(now).await?;
        if due.is_empty() {
            return Ok(());
        }
        debug!("refreshing chain capabilities for {} node(s)", due.len());

        let querier = &self.querier;
        let denom = self.denom.as_str();
        let ttl = self.ttl;
        let jitter = self.jitter;
        let concurrency = self.concurrency.max(1);

        let capabilities: Vec<NodeChainCapability> = futures::stream::iter(due)
            .map(|node| async move {
                match query_node(querier, &node, denom, ttl, jitter).await {
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
/// balance failure or an unparseable address skips the node so a stale value is left in place.
async fn query_node<C: NodeChainQuerier>(
    querier: &C,
    node: &NodeAwaitingCapabilityRefresh,
    denom: &str,
    ttl: Duration,
    jitter: Duration,
) -> anyhow::Result<NodeChainCapability> {
    let address = AccountId::from_str(&node.declared_chain_address).map_err(|e| {
        anyhow!(
            "node {} reported an invalid on-chain address: {e}",
            node.node_id
        )
    })?;

    let balance = querier.balance(&address, denom).await?;

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
        next_refresh_due_at: now + ttl + random_jitter(jitter),
    })
}

/// A random duration in `[0, jitter]`, added to each node's next-due time so that a population cached
/// together does not all fall due at the same instant.
fn random_jitter(jitter: Duration) -> Duration {
    let max_secs = jitter.as_secs();
    if max_secs == 0 {
        return Duration::ZERO;
    }
    Duration::from_secs(rand::thread_rng().gen_range(0..=max_secs))
}
