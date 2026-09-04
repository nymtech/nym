// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NodeStatusCache;
use crate::mixnet_contract_cache::cache::data::ConfigScoreData;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_performance::provider::{
    NodePerformanceProvider, NodesLivenessScores, NodesRoutingScores, NodesStressTestingScores,
};
use crate::node_status_api::cache::config_score::calculate_config_score;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::RefreshRequester;
use crate::support::caching::CacheNotificationWatcher;
use crate::support::nyxd::Client;
use crate::{
    mixnet_contract_cache::cache::MixnetContractCache,
    node_status_api::cache::NodeStatusCacheError, support::caching::CacheNotification,
};
use ::time::OffsetDateTime;
use cosmwasm_std::{coin, Coin};
use futures::StreamExt;
use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::{
    ChainInteractionCapabilitiesDetailed, DetailedNodePerformanceV2, NodeAnnotationV2,
};
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_task::ShutdownToken;
use nym_topology::CachedEpochRewardedSet;
use nym_validator_client::nyxd::module_traits::feegrant::query::FeegrantQueryClient;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::rpc::TendermintRpcClientExt;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tokio::time;
use tokio::time::Instant;
use tracing::{error, info, trace, warn};

pub(crate) struct NodeStatusCacheConfig {
    pub(crate) minimum_on_chain_balance: Coin,
    pub(crate) chain_capabilities_retrieval_concurrency: usize,

    /// How long a node's cached chain capabilities (balance + feegrant) stay valid before being
    /// re-queried. Evaluated per node, so lookups are spread out over time rather than refreshed
    /// in a single burst.
    pub(crate) chain_capabilities_refresh_interval: Duration,

    pub(crate) fallback_caching_interval: Duration,

    /// Specify whether external stress testing data should be used for calculating node performance
    /// score used for rewarding and active set selection
    /// note: this can only be enabled if use_performance_contract_data is set to false!
    pub(crate) use_stress_testing_data: bool,

    /// If `use_stress_testing_data` is set to true, this specifies the minimum % of nodes,
    /// that must have their stress data available in the `stress_testing_data_period`,
    /// in order to include that metric in performance calculation.
    /// This is done to protect against Network Monitor failures and not receiving any data.
    pub(crate) minimum_available_stress_testing_results: f32,

    /// If use_stress_testing_data is enabled, specifies the weight of the stress testing score in the overall performance score.
    pub(crate) stress_testing_score_weight: f64,

    /// Config score penalty for nodes that do not have a cosmos account capable of interacting with the nyx chain
    pub(crate) chain_interactions_penalty: f64,

    /// Specify whether liveness data should be folded into the node performance score. The score
    /// is annotated either way; this only controls whether it carries weight.
    pub(crate) use_liveness_data: bool,

    /// If `use_liveness_data` is set to true, this specifies the minimum % of liveness-eligible
    /// nodes that must have their liveness data available in the `liveness_data_period`,
    /// in order to include that metric in performance calculation.
    pub(crate) minimum_available_liveness_results: f32,

    /// If use_liveness_data is enabled, specifies the weight of the liveness score in the overall
    /// performance score. Defaults to zero.
    pub(crate) liveness_score_weight: f64,
}

/// A successfully-retrieved chain-capability lookup for a single node, tagged with the instant it
/// was fetched so its freshness can be evaluated against a TTL.
struct CachedChainCapabilities {
    capabilities: ChainInteractionCapabilitiesDetailed,
    fetched_at: Instant,
}

/// In-memory cache of successful chain-capability lookups, keyed by node id.
///
/// Only successful lookups are stored. Nodes that don't advertise a usable on-chain address, and
/// nodes whose query failed, are intentionally absent: they're cheaply re-derived from the
/// described data on every refresh and retried as needed, rather than being pinned to a stale
/// `false` for a whole TTL window. Entries for nodes that unbond or later drop their address are
/// evicted on the next refresh, so a stale value can't outlive the address it was derived from.
/// This map is purely in-memory and never persisted, so a restart simply triggers a one-off full
/// re-query on the first refresh.
#[derive(Default)]
struct ChainCapabilitiesCache {
    entries: HashMap<NodeId, CachedChainCapabilities>,
}

impl ChainCapabilitiesCache {
    /// Whether the node should be (re)queried: true if we have no cached value, or the cached value
    /// is older than `ttl`.
    fn needs_refresh(&self, node_id: NodeId, ttl: Duration) -> bool {
        match self.entries.get(&node_id) {
            None => true,
            Some(entry) => entry.fetched_at.elapsed() > ttl,
        }
    }

    /// Last known capabilities for a node, regardless of age. `None` if it has never been
    /// successfully queried (e.g. it advertises no address, or every query so far has failed).
    fn get(&self, node_id: NodeId) -> Option<ChainInteractionCapabilitiesDetailed> {
        self.entries
            .get(&node_id)
            .map(|entry| entry.capabilities.clone())
    }

    fn record(
        &mut self,
        node_id: NodeId,
        capabilities: ChainInteractionCapabilitiesDetailed,
        fetched_at: Instant,
    ) {
        self.entries.insert(
            node_id,
            CachedChainCapabilities {
                capabilities,
                fetched_at,
            },
        );
    }

    /// Drops every entry whose node id is not in `keep`. Used to evict nodes that have unbonded or
    /// no longer advertise a usable on-chain address, so their stale capabilities stop being served.
    fn retain_only(&mut self, keep: &HashSet<NodeId>) {
        self.entries.retain(|node_id, _| keep.contains(node_id));
    }
}

// Long running task responsible for keeping the node status cache up-to-date.
pub struct NodeStatusCacheRefresher {
    config: NodeStatusCacheConfig,

    /// Successful chain-capability lookups (balance + feegrant) cached per node, each with its own
    /// TTL so they're re-queried independently rather than all at once.
    chain_capabilities: ChainCapabilitiesCache,

    // Main stored data
    cache: NodeStatusCache,

    /// Query client for retrieving blockchain data
    query_client: QueryHttpRpcNyxdClient,

    // Sources for when refreshing data
    mixnet_contract_cache: MixnetContractCache,
    described_cache: SharedCache<DescribedNodes>,

    /// channel notifying us when mixnet cache has been refreshed,
    /// so that this cache could also be recreated
    mixnet_contract_cache_listener: CacheNotificationWatcher,

    /// channel notifying us when the describe cache has been refreshed,
    /// so that this cache could also be recreated
    describe_cache_listener: CacheNotificationWatcher,

    /// channel explicitly requesting cache refresh. it does not follow the usual rate limiting
    refresh_requester: RefreshRequester,

    /// Path to an on-disk location where the contents of the retrieved items should be written
    /// upon refresh
    on_disk_file: PathBuf,

    performance_provider: Box<dyn NodePerformanceProvider + Send + Sync>,
}

impl NodeStatusCacheRefresher {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        cache: NodeStatusCache,
        config: NodeStatusCacheConfig,
        chain_client: &Client,
        contract_cache: MixnetContractCache,
        described_cache: SharedCache<DescribedNodes>,
        contract_cache_listener: CacheNotificationWatcher,
        describe_cache_listener: CacheNotificationWatcher,
        performance_provider: Box<dyn NodePerformanceProvider + Send + Sync>,
        on_disk_file: PathBuf,
    ) -> Self {
        // due to the number of queries required, create an explicit query instance
        // of our nyxd client to avoid potentially blocking tasks requiring signing access
        let query_client = chain_client.query_client().await;

        Self {
            cache,
            config,
            chain_capabilities: ChainCapabilitiesCache::default(),
            mixnet_contract_cache: contract_cache,
            described_cache,
            mixnet_contract_cache_listener: contract_cache_listener,
            describe_cache_listener,
            refresh_requester: Default::default(),
            on_disk_file,
            performance_provider,
            query_client,
        }
    }

    pub(crate) fn refresh_requester(&self) -> RefreshRequester {
        self.refresh_requester.clone()
    }

    /// Runs the node status cache refresher task.
    pub async fn run(&mut self, shutdown_token: ShutdownToken) {
        let mut last_update = OffsetDateTime::now_utc();
        let mut fallback_interval = time::interval(self.config.fallback_caching_interval);
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    trace!("NodeStatusCacheRefresher: Received shutdown");
                    break;
                }
                // Update node status cache when the contract cache / describe cache is updated
                Ok(_) = self.mixnet_contract_cache_listener.changed() => {
                    tokio::select! {
                        _ = self.maybe_refresh(&mut fallback_interval, &mut last_update) => (),
                        _ = shutdown_token.cancelled() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                            break;
                        }
                    }
                }
                Ok(_) = self.describe_cache_listener.changed() => {
                    tokio::select! {
                        _ = self.maybe_refresh(&mut fallback_interval, &mut last_update) => (),
                        _ = shutdown_token.cancelled() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                            break;
                        }
                    }
                }
                // note: `Notify` is not cancellation safe, HOWEVER, there's only one listener,
                // so it doesn't matter if we lose our queue position
                _ = self.refresh_requester.notified() => {
                     tokio::select! {
                        // perform full refresh regardless of the rates
                        _ = self.refresh() => {
                            last_update = OffsetDateTime::now_utc();
                            fallback_interval.reset();
                        },
                        _ = shutdown_token.cancelled() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                            break;
                        }
                    }
                }


                // ... however, if we don't receive any notifications we fall back to periodic
                // refreshes
                _ = fallback_interval.tick() => {
                    tokio::select! {
                        _ = self.maybe_refresh(&mut fallback_interval, &mut last_update) => (),
                        _ = shutdown_token.cancelled() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                            break;
                        }
                    }
                }
            }
        }
        info!("NodeStatusCacheRefresher: Exiting");
    }

    fn caches_available(&self) -> bool {
        let contract_cache =
            *self.mixnet_contract_cache_listener.borrow() != CacheNotification::Start;
        let describe_cache = *self.describe_cache_listener.borrow() != CacheNotification::Start;

        let available = contract_cache && describe_cache;
        if !available {
            warn!(
                contract_cache,
                describe_cache, "auxiliary caches data is not yet available"
            )
        }

        available
    }

    async fn maybe_refresh(
        &mut self,
        fallback_interval: &mut time::Interval,
        last_updated: &mut OffsetDateTime,
    ) {
        if !self.caches_available() {
            trace!("not updating the cache since the auxiliary data is not yet available");
            return;
        }

        if OffsetDateTime::now_utc() - *last_updated < self.config.fallback_caching_interval {
            // don't update too often
            trace!("not updating the cache since they've been updated recently");
            return;
        }

        let _ = self.refresh().await;
        *last_updated = OffsetDateTime::now_utc();
        fallback_interval.reset();
    }

    /// Refreshes cached chain capabilities (balance + feegrant) for described nodes that need it:
    /// those with no cached value or whose value is older than the configured TTL. Nodes that
    /// don't advertise a usable on-chain address are skipped entirely - there's nothing to query,
    /// so we neither store nor retry them, and any value cached from a previously-advertised address
    /// is evicted. Only successful lookups are recorded; a failed query leaves any previous value
    /// untouched and is retried on the next refresh.
    // SAFETY: unwrap is fine as if the mutex got poisoned we'd be experiencing some UB anyway
    #[allow(clippy::unwrap_used)]
    async fn refresh_chain_capabilities(&mut self, nodes: &DescribedNodes) {
        // resolve the current usable on-chain account for every described node (parsing once so the
        // result is shared by both the eviction and the query paths below).
        let addressed = usable_chain_accounts(nodes);

        // evict cached entries for any node not in this set: those that have unbonded (the describe
        // cache only ever holds bonded nym-nodes) and those that no longer advertise a usable
        // address, so a stale value can't keep flowing into scoring after its address is gone.
        let usable_ids = addressed.iter().map(|(id, _)| *id).collect::<HashSet<_>>();
        self.chain_capabilities.retain_only(&usable_ids);

        let ttl = self.config.chain_capabilities_refresh_interval;
        let denom = self.config.minimum_on_chain_balance.denom.clone();

        // of the addressed nodes, query only those due a refresh (no cached value or past TTL).
        // materialised into a Vec so the immutable borrow on `self.chain_capabilities` ends before
        // the async queries (and before we record the results back into it).
        let to_query = addressed
            .into_iter()
            .filter(|(node_id, _)| self.chain_capabilities.needs_refresh(*node_id, ttl))
            .collect::<Vec<_>>();

        if to_query.is_empty() {
            return;
        }

        // note: we use `for_each_concurrent` rather than `stream::iter(..).buffer_unordered(..)`.
        // The latter yields a `Stream` whose `Send` bound gets over-generalised once chained into
        // `collect()`, tripping "implementation of `Send` is not general enough" (rust-lang/rust#102211)
        let concurrency = self.config.chain_capabilities_retrieval_concurrency.max(1);

        // std Mutex is fine because we don't hold it across await points
        let fresh = std::sync::Mutex::new(Vec::new());
        futures::stream::iter(to_query)
            .for_each_concurrent(concurrency, |(node_id, account_id)| {
                let denom = denom.clone();
                let query_client = &self.query_client;
                let fresh = &fresh;
                async move {
                    if let Some(caps) =
                        retrieve_chain_capabilities(query_client, node_id, account_id, denom).await
                    {
                        fresh.lock().unwrap().push((node_id, caps));
                    }
                }
            })
            .await;

        let now = Instant::now();
        for (node_id, caps) in fresh.into_inner().unwrap() {
            self.chain_capabilities.record(node_id, caps, now);
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn produce_node_annotations(
        &self,
        config_score_data: &ConfigScoreData,
        routing_scores: &NodesRoutingScores,
        stress_testing_scores: &NodesStressTestingScores,
        liveness_scores: &NodesLivenessScores,
        nym_nodes: &[NymNodeDetails],
        rewarded_set: &CachedEpochRewardedSet,
        described_nodes: &DescribedNodes,
    ) -> HashMap<NodeId, NodeAnnotationV2> {
        let mut annotations = HashMap::new();
        if nym_nodes.is_empty() {
            return annotations;
        }

        let minimum_balance = &self.config.minimum_on_chain_balance;
        let use_stress_testing_scores = self.config.use_stress_testing_data;
        let threshold = self.config.minimum_available_stress_testing_results;
        let chain_interactions_penalty = self.config.chain_interactions_penalty;

        // Each component's availability ratio is taken over ITS OWN eligible population, because
        // the two scopes differ: the orchestrator stress-tests only mixnodes but liveness-tests
        // anything it can classify (see `NodeType::from_roles`). Sharing one denominator would let
        // the network's mixnode:gateway composition - rather than orchestrator health - decide
        // whether either dataset is used at all.
        let stress_eligible_count = nym_nodes
            .iter()
            .filter(|n| stress_test_eligible(described_nodes.get_node(&n.node_id())))
            .count();
        let liveness_eligible_count = nym_nodes
            .iter()
            .filter(|n| liveness_eligible(described_nodes.get_node(&n.node_id())))
            .count();

        // Guard against an orchestrator outage silently slashing every eligible node's performance:
        // if too few nodes have a reachable sample for the configured window we assume the
        // orchestrator (rather than the network) is at fault and drop that component, falling back
        // towards the routing × config score.
        let include_stress_testing = self.component_applies(
            self.config.use_stress_testing_data,
            stress_testing_scores.available_count(),
            stress_eligible_count,
            self.config.minimum_available_stress_testing_results,
            "stress testing",
        );
        let include_liveness = self.component_applies(
            self.config.use_liveness_data,
            liveness_scores.available_count(),
            liveness_eligible_count,
            self.config.minimum_available_liveness_results,
            "liveness",
        );

        for nym_node in nym_nodes {
            let node_id = nym_node.node_id();
            let described = described_nodes.get_node(&node_id);
            let routing_score = routing_scores.get_or_log(node_id);
            let stress_testing_score = stress_testing_scores.get_or_log(node_id);
            let liveness_score = liveness_scores.get_or_log(node_id);
            let node_chain_cap = self.chain_capabilities.get(node_id);

            let config_score = calculate_config_score(
                minimum_balance,
                config_score_data,
                described,
                &node_chain_cap,
                chain_interactions_penalty,
            );

            // a node only takes a component if it is actually in scope for that component's test;
            // a node with no data by design must not be penalised for missing it.
            let components = PerformanceComponents {
                routing: routing_score.score,
                config: config_score.score,
                stress: (include_stress_testing && stress_test_eligible(described)).then_some(
                    WeightedComponent {
                        weight: self.config.stress_testing_score_weight,
                        score: stress_testing_score.score,
                    },
                ),
                liveness: (include_liveness && liveness_eligible(described)).then_some(
                    WeightedComponent {
                        weight: self.config.liveness_score_weight,
                        score: liveness_score.score,
                    },
                ),
            };

            annotations.insert(
                node_id,
                NodeAnnotationV2 {
                    current_role: rewarded_set.role(node_id).map(|r| r.into()),
                    chain_interaction_capabilities: node_chain_cap,
                    detailed_performance: DetailedNodePerformanceV2::new(
                        components.performance(),
                        routing_score,
                        config_score,
                        stress_testing_score,
                        // annotated whether or not it carried weight above, so the divergence
                        // between it and the routing score stays measurable while it is inert
                        liveness_score,
                    ),
                },
            );
        }

        annotations
    }

    /// Whether a component is enabled AND has enough of its eligible population covered to be
    /// trusted this refresh. Logs the shortfall, since an enabled component silently not applying
    /// is otherwise invisible.
    fn component_applies(
        &self,
        enabled: bool,
        available_count: usize,
        eligible_count: usize,
        threshold: f32,
        component: &str,
    ) -> bool {
        if !enabled {
            return false;
        }

        let ratio = availability_ratio(available_count, eligible_count);
        if ratio < threshold {
            info!(
                "not using {component} data for performance calculation: \
                 available ratio {ratio:.3} is below threshold {threshold:.3}"
            );
            return false;
        }

        true
    }

    /// Refreshes the node status cache by fetching the latest data from the contract cache
    #[allow(deprecated)]
    async fn refresh(&mut self) -> Result<(), NodeStatusCacheError> {
        info!("Updating node status cache");

        // Fetch contract cache data to work with
        let current_interval = self.mixnet_contract_cache.current_interval().await?;
        let rewarded_set = self.mixnet_contract_cache.rewarded_set_owned().await?;
        let nym_nodes = self.mixnet_contract_cache.nym_nodes().await;
        let config_score_data = self.mixnet_contract_cache.maybe_config_score_data().await?;

        // clone the cache handle (cheap Arc clone) so the read guard borrows the local rather than
        // `self`, leaving us free to take `&mut self` for the chain-capability refresh below.
        let described_cache = self.described_cache.clone();
        let Ok(described) = described_cache.get().await else {
            return Err(NodeStatusCacheError::UnavailableDescribedCache);
        };

        let all_ids = nym_nodes
            .iter()
            .map(|n| n.bond_information.node_id)
            .collect::<Vec<_>>();

        // note: any internal errors imply failures for that node in particular
        let routing_scores = self
            .performance_provider
            .get_batch_node_routing_scores(&all_ids, current_interval.current_epoch_absolute_id())
            .await?;

        let stress_testing_scores = self
            .performance_provider
            .get_batch_node_stress_testing_scores(
                &all_ids,
                current_interval.current_epoch_absolute_id(),
            )
            .await?;

        // fetched unconditionally, not gated on `use_liveness_data`: the score is annotated for
        // every node either way, which is what keeps the divergence gauge working while liveness
        // carries no weight
        let liveness_scores = self
            .performance_provider
            .get_batch_node_liveness_scores(&all_ids, current_interval.current_epoch_absolute_id())
            .await?;

        // refresh chain capabilities (balance + feegrant) for nodes that are due (new, previously
        // failed, or past their TTL), querying only the delta rather than the whole network.
        self.refresh_chain_capabilities(&described).await;

        // Create annotated data
        let node_annotations = self
            .produce_node_annotations(
                &config_score_data,
                &routing_scores,
                &stress_testing_scores,
                &liveness_scores,
                &nym_nodes,
                &rewarded_set,
                &described,
            )
            .await;

        // Update the cache
        self.cache.update(node_annotations).await;

        // attempt to update on-disk cache
        let Ok(new_cached) = self.cache.cache().await else {
            error!("the node status cache is still not initialised!");
            return Ok(());
        };
        // error reporting is handled by the serialise function itself
        let _ = new_cached.try_serialise_to_file(&self.on_disk_file);

        Ok(())
    }
}

/// Resolves the current usable on-chain account for each described node, returning `(node_id,
/// account_id)` pairs. Nodes that advertise no address (e.g. running an old version) or an
/// unparseable one are excluded - there's nothing to query for them.
fn usable_chain_accounts(nodes: &DescribedNodes) -> Vec<(NodeId, AccountId)> {
    nodes
        .nodes
        .values()
        .filter_map(|n| {
            let addr = n.description.auxiliary_details.address.as_ref()?;
            AccountId::from_str(addr)
                .inspect_err(|_| {
                    warn!("node {} has provided an invalid account address", n.node_id)
                })
                .ok()
                .map(|account_id| (n.node_id, account_id))
        })
        .collect()
}

async fn retrieve_chain_capabilities(
    query_client: &QueryHttpRpcNyxdClient,
    node_id: NodeId,
    account_id: AccountId,
    balance_denom: String,
) -> Option<ChainInteractionCapabilitiesDetailed> {
    let on_chain_balance = match query_client
        .get_balance(&account_id, balance_denom.clone())
        .await
    {
        Ok(balance) => balance.map(Into::into).unwrap_or(coin(0, balance_denom)),
        Err(err) => {
            warn!(node_id, %err, "failed to retrieve node balance");
            return None;
        }
    };

    let is_feegrant_grantee = match query_client.allowances(account_id, None).await {
        // currently this is a very coarse check. the grant might be expired, it might not allow for
        // cosmwasm executemsg, but that's a good enough first iteration
        Ok(allowances) => !allowances.allowances.is_empty(),
        Err(err) => {
            warn!(node_id, %err, "failed to retrieve node feegrant allowances");
            // if there was a network blip, at least preserve the balance information
            false
        }
    };

    Some(ChainInteractionCapabilitiesDetailed {
        on_chain_balance,
        is_feegrant_grantee,
    })
}

/// Whether `node` is currently in scope for stress testing, and therefore expected to have a
/// stress-test sample. This is the single source of truth for stress-test scope and must stay in
/// sync with the orchestrator's test-target selection (`NodeType::from_roles`, which keys off the
/// self-described role capabilities).
///
/// A node that is *not* in scope has no stress data by design - not because it failed a test - so
/// it must never have the stress component folded into its performance score (otherwise gateways
/// would be silently penalised for a test they were never subjected to). Conversely, an in-scope
/// node with no sample legitimately scores 0 for stress, guarded network-wide by the availability
/// threshold against orchestrator outages.
///
/// Today only mixnodes are stress-tested; when gateway stress testing lands, widen this predicate
/// (e.g. to also accept `entry`/`exit` capable nodes) and nothing else in the scoring path needs
/// to change.
fn stress_test_eligible(described: Option<&NymNodeDescriptionV3>) -> bool {
    described
        .map(|n| n.description.declared_role.mixnode)
        .unwrap_or(false)
}

/// Whether `node` is currently in scope for LIVENESS testing, and therefore expected to have a
/// liveness sample. Wider than [`stress_test_eligible`]: liveness probes gateways too, as a
/// mixing hop, as a gateway, or both for a dual-role node.
///
/// Mirrors the orchestrator's `NodeType::from_roles`, which classifies on
/// `(mixnode_enabled, gateway_enabled)` and yields `Unknown` - ineligible for every kind - only
/// when neither is set. `declared_role.entry` is that same `gateway_enabled` flag, mapped in
/// `type_translation.rs`. A node that has never answered its self-description is out of scope,
/// matching `Unknown`, because the orchestrator cannot classify it either.
fn liveness_eligible(described: Option<&NymNodeDescriptionV3>) -> bool {
    described
        .map(|n| n.description.declared_role.mixnode || n.description.declared_role.entry)
        .unwrap_or(false)
}

/// Fraction of eligible nodes for which the orchestrator produced a reachable sample.
/// The denominator is the eligible count, not the total node count, so the network's role
/// composition cannot drag the ratio below the orchestrator-health threshold. Returns 0 when there
/// are no eligible nodes (nothing to base a judgement on, so the data is treated as unavailable).
///
/// Shared by both components, each passing its OWN eligible population: stress counts only
/// mixnodes, liveness counts anything the orchestrator can classify.
fn availability_ratio(available_count: usize, eligible_count: usize) -> f32 {
    if eligible_count == 0 {
        0.0
    } else {
        available_count as f32 / eligible_count as f32
    }
}

/// One measured component and the weight it carries in the overall performance figure.
#[derive(Clone, Copy)]
struct WeightedComponent {
    weight: f64,
    score: f64,
}

/// The inputs to a node's overall performance figure.
///
/// A component that does not apply to this node is `None` rather than a zero weight, so "not
/// measured" cannot be confused with "measured and scored zero" - the difference matters, because
/// an out-of-scope node must be scored as if the component did not exist rather than take a
/// weighted zero for a test it was never subjected to.
struct PerformanceComponents {
    routing: f64,
    config: f64,
    stress: Option<WeightedComponent>,
    liveness: Option<WeightedComponent>,
}

impl PerformanceComponents {
    /// Overall node performance: a weighted arithmetic mean in which each applied component
    /// contributes its own weight and routing × config takes whatever weight is left over.
    ///
    /// A mean rather than a product, so one zero component does not zero the whole figure. With no
    /// component applied this is exactly routing × config, which is what makes a zero-weighted
    /// liveness component leave every node's performance untouched.
    ///
    /// The leftover weight cannot go negative in practice: config validation rejects weights that
    /// sum above 1.0, which is enforced there rather than clamped here so that a misconfiguration
    /// fails at startup instead of silently scoring every node differently than intended.
    fn performance(&self) -> f64 {
        let applied = [self.stress, self.liveness];
        let applied = applied.iter().flatten();

        let (weighted_total, applied_weight) = applied.fold((0.0, 0.0), |(total, weight), c| {
            (total + c.weight * c.score, weight + c.weight)
        });

        weighted_total + (1.0 - applied_weight) * self.routing * self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_api_requests::models::described::v3::mock_nym_node_description;

    const ROUTING: f64 = 0.9;
    const CONFIG: f64 = 1.0;

    fn components(
        stress: Option<WeightedComponent>,
        liveness: Option<WeightedComponent>,
    ) -> PerformanceComponents {
        PerformanceComponents {
            routing: ROUTING,
            config: CONFIG,
            stress,
            liveness,
        }
    }

    fn weighted(weight: f64, score: f64) -> Option<WeightedComponent> {
        Some(WeightedComponent { weight, score })
    }

    #[test]
    fn ineligible_nodes_are_not_penalised_for_missing_stress_data() {
        let sw = 0.2;
        let stress = 0.0; // no stress sample -> unreachable() score

        // an out-of-scope node (e.g. a gateway) is scored on routing × config alone - no haircut
        let out_of_scope = components(None, None).performance();
        assert_eq!(out_of_scope, ROUTING * CONFIG);

        // an in-scope node (a mixnode) with no/zero stress sample does take the weighted hit
        let in_scope = components(weighted(sw, stress), None).performance();
        assert_eq!(in_scope, sw * stress + (1.0 - sw) * ROUTING * CONFIG);
        assert!(
            in_scope < out_of_scope,
            "in-scope node with a 0 stress score should score strictly lower than an out-of-scope one"
        );
    }

    /// The property that lets liveness ship applied-but-inert: with weight zero it cannot move any
    /// node's performance, whatever it scored. If this ever fails, every node in the two
    /// populations that legitimately score zero during rollout is silently being penalised.
    #[test]
    fn a_zero_weight_liveness_component_leaves_performance_unchanged() {
        let baseline = components(None, None).performance();

        for score in [0.0, 0.5, 1.0] {
            assert_eq!(
                components(None, weighted(0.0, score)).performance(),
                baseline,
                "liveness at zero weight must not move the score, even scoring {score}"
            );
        }

        // and it stays inert alongside an applied stress component
        let stress = weighted(0.2, 0.5);
        assert_eq!(
            components(stress, weighted(0.0, 0.0)).performance(),
            components(stress, None).performance()
        );
    }

    /// The converse, so the test above cannot pass merely because the component is never wired in.
    #[test]
    fn a_weighted_liveness_component_moves_the_score() {
        let baseline = components(None, None).performance();
        let lw = 0.3;

        let dead = components(None, weighted(lw, 0.0)).performance();
        assert_eq!(dead, (1.0 - lw) * ROUTING * CONFIG);
        assert!(dead < baseline, "a zero liveness score must cost the node");

        let perfect = components(None, weighted(lw, 1.0)).performance();
        assert_eq!(perfect, lw + (1.0 - lw) * ROUTING * CONFIG);
        assert!(
            perfect > baseline,
            "a perfect liveness score must lift a node whose routing is imperfect"
        );
    }

    /// Both components applied share one weight budget with routing × config, which takes the
    /// remainder. Config validation is what stops that remainder going negative.
    #[test]
    fn applied_components_share_the_weight_budget_with_routing() {
        let (sw, lw) = (0.2, 0.3);

        let both = components(weighted(sw, 1.0), weighted(lw, 1.0)).performance();
        assert_eq!(both, sw + lw + (1.0 - sw - lw) * ROUTING * CONFIG);

        // a full budget leaves routing × config no weight at all
        let saturated = components(weighted(0.5, 1.0), weighted(0.5, 1.0)).performance();
        assert_eq!(saturated, 1.0);
    }

    #[test]
    fn availability_ratio_uses_eligible_denominator() {
        // every eligible node reachable -> full ratio, no matter how many ineligible nodes
        // (gateways) also exist in the network.
        assert_eq!(availability_ratio(5, 5), 1.0);
        // half the eligible nodes reachable
        assert_eq!(availability_ratio(3, 6), 0.5);
        // no eligible nodes -> 0, never a division by zero / NaN
        assert_eq!(availability_ratio(0, 0), 0.0);
    }

    /// Liveness scope is strictly wider than stress scope, which is the whole reason the two
    /// components need separate eligibility predicates and separate availability denominators.
    #[test]
    fn liveness_scope_covers_gateways_and_stress_scope_does_not() {
        let mut mixnode = mock_nym_node_description(1);
        mixnode.description.declared_role.mixnode = true;
        mixnode.description.declared_role.entry = false;
        assert!(stress_test_eligible(Some(&mixnode)));
        assert!(liveness_eligible(Some(&mixnode)));

        let mut gateway = mock_nym_node_description(2);
        gateway.description.declared_role.mixnode = false;
        gateway.description.declared_role.entry = true;
        assert!(!stress_test_eligible(Some(&gateway)));
        assert!(
            liveness_eligible(Some(&gateway)),
            "a gateway is liveness-tested even though it is never stress-tested"
        );

        let mut dual = mock_nym_node_description(3);
        dual.description.declared_role.mixnode = true;
        dual.description.declared_role.entry = true;
        assert!(stress_test_eligible(Some(&dual)));
        assert!(liveness_eligible(Some(&dual)));

        // neither role declared: the orchestrator classifies this as `Unknown` and assigns it
        // nothing, so it must not be counted in either denominator
        let mut unclassified = mock_nym_node_description(4);
        unclassified.description.declared_role.mixnode = false;
        unclassified.description.declared_role.entry = false;
        assert!(!stress_test_eligible(Some(&unclassified)));
        assert!(!liveness_eligible(Some(&unclassified)));

        // and a node that never answered its self-description at all
        assert!(!stress_test_eligible(None));
        assert!(!liveness_eligible(None));
    }

    #[test]
    fn only_in_scope_node_types_are_stress_test_eligible() {
        let mut mixnode = mock_nym_node_description(1);
        mixnode.description.declared_role.mixnode = true;
        assert!(stress_test_eligible(Some(&mixnode)));

        // a gateway-only node (not a mixnode) is currently out of scope
        let mut gateway = mock_nym_node_description(2);
        gateway.description.declared_role.mixnode = false;
        assert!(!stress_test_eligible(Some(&gateway)));

        // a node with no self-described data is out of scope (the orchestrator can't classify it)
        assert!(!stress_test_eligible(None));
    }

    fn described_nodes(nodes: impl IntoIterator<Item = NymNodeDescriptionV3>) -> DescribedNodes {
        DescribedNodes {
            nodes: nodes.into_iter().map(|n| (n.node_id, n)).collect(),
            addresses_cache: HashMap::new(),
        }
    }

    #[test]
    fn cached_capabilities_are_evicted_when_a_node_loses_its_usable_address() {
        let (with_addr, without_addr, unbonded) = (1, 2, 3);

        let mut keeps_address = mock_nym_node_description(0);
        keeps_address.node_id = with_addr;

        // still bonded/described, but its self-described address is gone (e.g. downgraded to a
        // version that only exposes v1 auxiliary details)
        let mut drops_address = mock_nym_node_description(1);
        drops_address.node_id = without_addr;
        drops_address.description.auxiliary_details.address = None;

        let described = described_nodes([keeps_address, drops_address]);

        let mut cache = ChainCapabilitiesCache::default();
        let caps = ChainInteractionCapabilitiesDetailed {
            on_chain_balance: coin(1_000000, "unym"),
            is_feegrant_grantee: true,
        };
        let now = Instant::now();
        cache.record(with_addr, caps.clone(), now);
        cache.record(without_addr, caps.clone(), now);
        cache.record(unbonded, caps, now); // no longer in the describe cache at all

        let usable_ids = usable_chain_accounts(&described)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<HashSet<_>>();
        cache.retain_only(&usable_ids);

        assert!(cache.get(with_addr).is_some()); // still advertises a usable address -> kept
        assert!(cache.get(without_addr).is_none()); // address dropped -> evicted
        assert!(cache.get(unbonded).is_none()); // no longer described -> evicted
    }
}
