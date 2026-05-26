// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NodeStatusCache;
use crate::mixnet_contract_cache::cache::data::ConfigScoreData;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_performance::provider::{
    NodePerformanceProvider, NodesRoutingScores, NodesStressTestingScores,
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
use cosmwasm_std::Coin;
use futures::StreamExt;
use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::{DetailedNodePerformanceV2, NodeAnnotationV2};
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_task::ShutdownToken;
use nym_topology::CachedEpochRewardedSet;
use nym_validator_client::nyxd::{AccountId, CosmWasmClient};
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tokio::time;
use tokio::time::Instant;
use tracing::{error, info, trace, warn};

pub(crate) struct NodeStatusCacheConfig {
    pub(crate) minimum_on_chain_balance: Coin,
    pub(crate) balance_retrieval_concurrency: usize,

    /// Indicates how often should the chain balances of known nodes be refreshed.
    /// (it is an overkill to do it every single iteration)
    pub(crate) chain_balances_refresh_interval: Duration,

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
}

// Long running task responsible for keeping the node status cache up-to-date.
pub struct NodeStatusCacheRefresher {
    config: NodeStatusCacheConfig,

    /// Indicates the last time chain balances of known nodes were refreshed.
    last_refreshed_chain_balances: Option<Instant>,

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
            last_refreshed_chain_balances: None,
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

    // SAFETY: unwrap is fine as if the mutex got poisoned we'd be experiencing some UB anyway
    #[allow(clippy::unwrap_used)]
    async fn retrieve_balances(
        &self,
        nodes: &DescribedNodes,
    ) -> Result<HashMap<NodeId, Option<Coin>>, NodeStatusCacheError> {
        let denom = self.config.minimum_on_chain_balance.denom.clone();

        // create an iterator of node ids with valid associated account addresses
        let to_check = nodes.nodes.values().filter_map(|n| {
            n.description
                .auxiliary_details
                .address
                .as_ref()
                .and_then(|addr| {
                    AccountId::from_str(addr)
                        .inspect_err(|_| {
                            warn!("node {} has provided an invalid account address", n.node_id)
                        })
                        .ok()
                        .map(|account_id| (n.node_id, account_id))
                })
        });

        // note: we use `for_each_concurrent` rather than `stream::iter(..).buffer_unordered(..)`.
        // The latter yields a `Stream` whose `Send` bound gets over-generalised once chained into
        // `collect()`, tripping "implementation of `Send` is not general enough" (rust-lang/rust#102211)
        let concurrency = self.config.balance_retrieval_concurrency.max(1);

        // std Mutex is fine because we don't hold it across await points
        let balances = std::sync::Mutex::new(HashMap::<NodeId, Option<Coin>>::new());
        futures::stream::iter(to_check)
            .for_each_concurrent(concurrency, |(node_id, account_id)| {
                let denom = denom.clone();
                let query_client = &self.query_client;
                let balances = &balances;
                async move {
                    match query_client.get_balance(&account_id, denom).await {
                        Ok(balance) => {
                            balances
                                .lock()
                                .unwrap()
                                .insert(node_id, balance.map(Into::into));
                        }
                        Err(err) => {
                            warn!(node_id, %err, "failed to retrieve node balance");
                        }
                    }
                }
            })
            .await;
        let balances = balances.into_inner().unwrap();

        Ok(balances)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn produce_node_annotations(
        &self,
        config_score_data: &ConfigScoreData,
        routing_scores: &NodesRoutingScores,
        stress_testing_scores: &NodesStressTestingScores,
        nym_nodes: &[NymNodeDetails],
        rewarded_set: &CachedEpochRewardedSet,
        described_nodes: &DescribedNodes,
        balances: HashMap<NodeId, Option<Coin>>,
    ) -> HashMap<NodeId, NodeAnnotationV2> {
        let mut annotations = HashMap::new();
        if nym_nodes.is_empty() {
            return annotations;
        }

        let minimum_balance = &self.config.minimum_on_chain_balance;
        let use_stress_testing_scores = self.config.use_stress_testing_data;
        let threshold = self.config.minimum_available_stress_testing_results;

        // Only mixnodes are currently stress-tested: the orchestrator selects test targets by
        // self-described mixnode capability (see `NodeType::from_roles`), so the availability ratio
        // must be taken over stress-test-eligible nodes only. Counting gateways in the denominator
        // would let the network's mixnode:gateway composition - rather than orchestrator health -
        // decide whether the data is used at all.
        let eligible_count = nym_nodes
            .iter()
            .filter(|n| stress_test_eligible(described_nodes.get_node(&n.node_id())))
            .count();
        let available_ratio =
            stress_availability_ratio(stress_testing_scores.available_count(), eligible_count);

        // Guard against an orchestrator outage silently slashing every eligible node's performance:
        // if too few mixnodes have a reachable stress-test sample for the configured window we
        // assume the orchestrator (rather than the network) is at fault and fall back to the
        // routing × config score alone.
        let include_stress_testing = use_stress_testing_scores && available_ratio >= threshold;

        if use_stress_testing_scores && !include_stress_testing {
            info!(
                "not using stress testing data for performance calculation: \
                 available ratio {available_ratio:.3} is below threshold {threshold:.3}"
            );
        }

        for nym_node in nym_nodes {
            let node_id = nym_node.node_id();
            let described = described_nodes.get_node(&node_id);
            let routing_score = routing_scores.get_or_log(node_id);
            let stress_testing_score = stress_testing_scores.get_or_log(node_id);
            let on_chain_balance = balances.get(&node_id).unwrap_or(&None).clone();

            let config_score = calculate_config_score(
                minimum_balance,
                config_score_data,
                described,
                &on_chain_balance,
            );

            // a node only takes the stress-testing component if it is actually stress-tested (i.e.
            // it is a mixnode); gateways have no stress data and must not be penalised for it.
            let apply_stress = include_stress_testing && stress_test_eligible(described);
            let performance = node_performance(
                apply_stress,
                self.config.stress_testing_score_weight,
                stress_testing_score.score,
                routing_score.score,
                config_score.score,
            );

            annotations.insert(
                node_id,
                NodeAnnotationV2 {
                    current_role: rewarded_set.role(node_id).map(|r| r.into()),
                    on_chain_balance,
                    detailed_performance: DetailedNodePerformanceV2::new(
                        performance,
                        routing_score,
                        config_score,
                        stress_testing_score,
                    ),
                },
            );
        }

        annotations
    }

    fn should_refresh_balances(&self) -> bool {
        let Some(last_refresh) = self.last_refreshed_chain_balances else {
            return true;
        };
        last_refresh.elapsed() > self.config.chain_balances_refresh_interval
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

        let Ok(described) = self.described_cache.get().await else {
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

        // decide whether to refresh cache of node balances

        let balances = if self.should_refresh_balances() {
            let balances = self.retrieve_balances(&described).await?;
            self.last_refreshed_chain_balances = Some(Instant::now());
            balances
        } else {
            // use the currently cached values instead
            self.cache.node_balances().await?
        };

        // Create annotated data
        let node_annotations = self
            .produce_node_annotations(
                &config_score_data,
                &routing_scores,
                &stress_testing_scores,
                &nym_nodes,
                &rewarded_set,
                &described,
                balances,
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

/// Fraction of stress-test-eligible nodes for which the orchestrator produced a reachable sample.
/// The denominator is the eligible count, not the total node count, so the network's role
/// composition cannot drag the ratio below the orchestrator-health threshold. Returns 0 when there
/// are no eligible nodes (nothing to base a judgement on, so the data is treated as unavailable).
fn stress_availability_ratio(available_count: usize, eligible_count: usize) -> f32 {
    if eligible_count == 0 {
        0.0
    } else {
        available_count as f32 / eligible_count as f32
    }
}

/// Overall node performance. When the stress-testing component applies, it is a weighted arithmetic
/// mean of the stress score and routing × config (so a single 0 doesn't zero the whole thing);
/// otherwise it is simply routing × config.
fn node_performance(
    apply_stress: bool,
    stress_weight: f64,
    stress_score: f64,
    routing_score: f64,
    config_score: f64,
) -> f64 {
    if apply_stress {
        stress_weight * stress_score + (1.0 - stress_weight) * routing_score * config_score
    } else {
        routing_score * config_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_api_requests::models::mock_nym_node_description;

    #[test]
    fn ineligible_nodes_are_not_penalised_for_missing_stress_data() {
        let sw = 0.2;
        let stress = 0.0; // no stress sample -> unreachable() score
        let routing = 0.9;
        let config = 1.0;

        // an out-of-scope node (e.g. a gateway) is scored on routing × config alone - no haircut
        let out_of_scope = node_performance(false, sw, stress, routing, config);
        assert_eq!(out_of_scope, routing * config);

        // an in-scope node (a mixnode) with no/zero stress sample does take the weighted hit
        let in_scope = node_performance(true, sw, stress, routing, config);
        assert_eq!(in_scope, sw * stress + (1.0 - sw) * routing * config);
        assert!(
            in_scope < out_of_scope,
            "in-scope node with a 0 stress score should score strictly lower than an out-of-scope one"
        );
    }

    #[test]
    fn availability_ratio_uses_eligible_denominator() {
        // every eligible node reachable -> full ratio, no matter how many ineligible nodes
        // (gateways) also exist in the network.
        assert_eq!(stress_availability_ratio(5, 5), 1.0);
        // half the eligible nodes reachable
        assert_eq!(stress_availability_ratio(3, 6), 0.5);
        // no eligible nodes -> 0, never a division by zero / NaN
        assert_eq!(stress_availability_ratio(0, 0), 0.0);
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
}
