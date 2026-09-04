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
use crate::support::config::PerformanceProviderScoring;
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
use tracing::{debug, error, info, trace, warn};

pub(crate) struct NodeStatusCacheConfig {
    pub(crate) minimum_on_chain_balance: Coin,
    pub(crate) chain_capabilities_retrieval_concurrency: usize,

    /// How long a node's cached chain capabilities (balance + feegrant) stay valid before being
    /// re-queried. Evaluated per node, so lookups are spread out over time rather than refreshed
    /// in a single burst.
    pub(crate) chain_capabilities_refresh_interval: Duration,

    pub(crate) fallback_caching_interval: Duration,

    /// Which properties contribute to a node's score, and in what proportion. Their enabled
    /// weights sum to 1.0; whatever actually applies to a given node is renormalised.
    pub(crate) scoring: PerformanceProviderScoring,

    /// If stress testing is enabled, this specifies the minimum % of nodes,
    /// that must have their stress data available in the `stress_testing_data_period`,
    /// in order to include that metric in performance calculation.
    /// This is done to protect against Network Monitor failures and not receiving any data.
    pub(crate) minimum_available_stress_testing_results: f32,

    /// Config score penalty for nodes that do not have a cosmos account capable of interacting with the nyx chain
    pub(crate) chain_interactions_penalty: f64,

    /// If liveness is enabled, this specifies the minimum % of liveness-eligible
    /// nodes that must have their liveness data available in the `liveness_data_period`,
    /// in order to include that metric in performance calculation.
    pub(crate) minimum_available_liveness_results: f32,
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
        let scoring = &self.config.scoring;
        let include_stress_testing = self.component_applies(
            scoring.stress_testing.enabled,
            stress_testing_scores.available_count(),
            stress_eligible_count,
            self.config.minimum_available_stress_testing_results,
            "stress testing",
        );
        let include_liveness = self.component_applies(
            scoring.liveness.enabled,
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

            // a node only takes a property if it is actually in scope for that property's test; a
            // node with no data BY DESIGN must not be penalised for missing it, which is what
            // renormalising over the applied set achieves - a gateway, never stress-tested, is
            // scored on what it does have rather than docked the stress share.
            let components = PerformanceComponents {
                config: config_score.score,
                legacy_v1_routing: scoring
                    .legacy_v1_routing
                    .enabled
                    .then_some(WeightedComponent {
                        weight: scoring.legacy_v1_routing.weight,
                        score: routing_score.score,
                    }),
                stress: (include_stress_testing && stress_test_eligible(described)).then_some(
                    WeightedComponent {
                        weight: scoring.stress_testing.weight,
                        score: stress_testing_score.score,
                    },
                ),
                liveness: (include_liveness && liveness_eligible(described)).then_some(
                    WeightedComponent {
                        weight: scoring.liveness.weight,
                        score: liveness_score.score,
                    },
                ),
            };

            // Nothing applied, so there is no delivery measurement for this node and no score to
            // publish. Leaving it out of the map keeps whatever it was last annotated with rather
            // than inventing a figure: a zero would slash it for a gap that is ours, and anything
            // else would reward it for nothing measured.
            let Some(performance) = components.performance() else {
                debug!(
                    node_id,
                    "no scoring property applied to this node, leaving its annotation unchanged"
                );
                continue;
            };

            annotations.insert(
                node_id,
                NodeAnnotationV2 {
                    current_role: rewarded_set.role(node_id).map(|r| r.into()),
                    chain_interaction_capabilities: node_chain_cap,
                    detailed_performance: DetailedNodePerformanceV2::new(
                        performance,
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
/// A property that does not apply to this node is `None` rather than a zero weight, so "not
/// measured" cannot be confused with "measured and scored zero" - the difference matters, because
/// an out-of-scope node must be scored as if the property did not exist rather than take a
/// weighted zero for a test it was never subjected to.
struct PerformanceComponents {
    /// Multiplies the delivery figure rather than competing with it for weight. Configuration is a
    /// GATE on how well a node carries traffic, not another measurement of it, so it applies to
    /// every property equally.
    config: f64,
    legacy_v1_routing: Option<WeightedComponent>,
    stress: Option<WeightedComponent>,
    liveness: Option<WeightedComponent>,
}

impl PerformanceComponents {
    /// Overall node performance: the weighted mean of whichever delivery properties APPLIED,
    /// renormalised over their weights, multiplied by the config score.
    ///
    /// Renormalisation is what keeps a node from being docked for a measurement it could never
    /// have. A gateway is never stress-tested, so with routing at 0.7 and stress at 0.3 it is
    /// scored `routing * config` rather than `0.7 * routing * config`; the same protects every node
    /// when an availability threshold drops a property mid-flight because an orchestrator is down.
    /// It does mean effective weights differ by role, so a declared weight is a proportion of
    /// whatever applies rather than a fixed share.
    ///
    /// A mean rather than a product, so one zero property does not zero the whole figure. With a
    /// single property applied at any weight this is exactly `that_score * config`, which is the
    /// identity that makes the legacy `routing * config` behaviour recoverable.
    ///
    /// `None` when NOTHING applied: there is no delivery measurement, and inventing one either way
    /// would be wrong - zero slashes a node for our own outage, one rewards it for nothing.
    fn performance(&self) -> Option<f64> {
        let applied = [self.legacy_v1_routing, self.stress, self.liveness];

        let (weighted_total, applied_weight) = applied
            .iter()
            .flatten()
            .fold((0.0, 0.0), |(total, weight), c| {
                (total + c.weight * c.score, weight + c.weight)
            });

        if applied_weight <= 0.0 {
            return None;
        }

        Some(weighted_total / applied_weight * self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_api_requests::models::described::v3::mock_nym_node_description;

    /// The production split: legacy v1 routing at 0.7 alongside stress testing at 0.3.
    const RW: f64 = 0.7;
    const SW: f64 = 0.3;

    fn scored(
        config: f64,
        legacy_v1_routing: Option<WeightedComponent>,
        stress: Option<WeightedComponent>,
        liveness: Option<WeightedComponent>,
    ) -> PerformanceComponents {
        PerformanceComponents {
            config,
            legacy_v1_routing,
            stress,
            liveness,
        }
    }

    fn weighted(weight: f64, score: f64) -> Option<WeightedComponent> {
        Some(WeightedComponent { weight, score })
    }

    /// Renormalisation divides, so results carry rounding: `0.7 * 0.8 / 0.7 * 0.5` is
    /// `0.39999999999999997`, not `0.4`. Exact equality would test the bit pattern rather than the
    /// arithmetic, so these compare within a tolerance far tighter than any weight an operator
    /// could set.
    #[allow(clippy::unwrap_used)]
    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("expected a score");
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    /// The identity that makes the legacy behaviour recoverable: one property applied, at ANY
    /// weight, is that property's score times the config score.
    fn a_single_property_reproduces_score_times_config() {
        for weight in [0.3, 0.7, 1.0] {
            assert_close(
                scored(0.5, weighted(weight, 0.8), None, None).performance(),
                0.8 * 0.5,
            );
        }
    }

    /// The gateway case. Stress is declared at 0.3 but never applies to a gateway, so the gateway
    /// must be scored on what it does have rather than docked the missing 0.3.
    #[test]
    fn a_node_out_of_scope_for_a_property_is_not_docked_its_share() {
        let gateway = scored(1.0, weighted(RW, 0.9), None, None).performance();
        assert_eq!(gateway, Some(0.9), "renormalised to routing alone");

        // the mixnode beside it does take both, at face value since they sum to one
        let mixnode = scored(1.0, weighted(RW, 0.9), weighted(SW, 0.5), None).performance();
        assert_eq!(mixnode, Some(RW * 0.9 + SW * 0.5));

        assert!(
            gateway > mixnode,
            "the gateway must not be penalised for a test it cannot take"
        );
    }

    /// The defect the restructure exists to fix: config gates EVERY property, not just the legacy
    /// one. Under the old formula the stress share escaped the gate entirely.
    #[test]
    fn the_config_score_gates_every_property() {
        let config = 0.5;
        let perfect_delivery =
            scored(config, weighted(RW, 1.0), weighted(SW, 1.0), None).performance();

        // all delivery perfect, so the whole figure is exactly the config score
        assert_eq!(perfect_delivery, Some(config));

        // and the old formula's escape hatch is gone: a perfect stress score cannot lift a node
        // above its config ceiling
        assert!(
            perfect_delivery <= Some(config),
            "no property may exceed the config gate"
        );
    }

    /// A dual-role node's liveness carries a larger share than a mixnode's, because it has fewer
    /// properties to divide the measurement with. That is renormalisation working, not a bug.
    #[test]
    fn effective_weights_depend_on_what_applied() {
        // routing 0.5 / stress 0.3 / liveness 0.2 declared
        let (rw, sw, lw) = (0.5, 0.3, 0.2);

        let mixnode = scored(1.0, weighted(rw, 1.0), weighted(sw, 0.0), weighted(lw, 0.0))
            .performance()
            .unwrap();
        // stress does not apply, so routing and liveness split the whole measurement
        let gateway = scored(1.0, weighted(rw, 1.0), None, weighted(lw, 0.0))
            .performance()
            .unwrap();

        assert_eq!(mixnode, rw);
        assert_eq!(gateway, rw / (rw + lw));
        assert!(
            gateway > mixnode,
            "with stress absent the surviving properties carry proportionally more"
        );
    }

    /// The property that lets liveness ship applied-but-inert: enabled at weight zero it cannot
    /// move any node's score, whatever it measured. If this ever fails, every node in the two
    /// populations that legitimately score zero during rollout is being penalised for a rollout
    /// still in progress.
    #[test]
    fn a_zero_weight_liveness_property_leaves_performance_unchanged() {
        let routing = weighted(RW, 0.9);
        let baseline = scored(0.8, routing, None, None).performance();

        for score in [0.0, 0.5, 1.0] {
            assert_eq!(
                scored(0.8, routing, None, weighted(0.0, score)).performance(),
                baseline,
                "liveness at zero weight must not move the score, even measuring {score}"
            );
        }

        // and it stays inert beside an applied stress property too
        let stress = weighted(SW, 0.5);
        assert_eq!(
            scored(0.8, routing, stress, weighted(0.0, 0.0)).performance(),
            scored(0.8, routing, stress, None).performance()
        );
    }

    /// Nothing applied means no delivery measurement, so no score - inventing one would either
    /// slash a node for our outage or reward it for nothing.
    #[test]
    fn no_applied_property_yields_no_score() {
        assert_eq!(scored(1.0, None, None, None).performance(), None);

        // a property present but at zero weight contributes nothing and cannot rescue it
        assert_eq!(
            scored(1.0, weighted(0.0, 1.0), None, None).performance(),
            None
        );
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
