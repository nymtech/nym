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
use crate::{
    mixnet_contract_cache::cache::MixnetContractCache,
    node_status_api::cache::NodeStatusCacheError, support::caching::CacheNotification,
};
use ::time::OffsetDateTime;
use nym_api_requests::models::{DetailedNodePerformanceV2, NodeAnnotationV2};
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_task::ShutdownToken;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, trace, warn};

pub(crate) struct NodeStatusCacheConfig {
    pub(crate) fallback_caching_interval: Duration,

    /// Specify whether external stress testing data should be used for calculating node performance
    /// score used for rewarding and active set selection
    /// note: this can only be enabled if use_performance_contract_data is set to false!
    pub use_stress_testing_data: bool,

    /// If `use_stress_testing_data` is set to true, this specifies the minimum % of nodes,
    /// that must have their stress data available in the `stress_testing_data_period`,
    /// in order to include that metric in performance calculation.
    /// This is done to protect against Network Monitor failures and not receiving any data.
    pub minimum_available_stress_testing_results: f32,

    /// If use_stress_testing_data is enabled, specifies the weight of the stress testing score in the overall performance score.
    pub stress_testing_score_weight: f64,
}

// Long running task responsible for keeping the node status cache up-to-date.
pub struct NodeStatusCacheRefresher {
    config: NodeStatusCacheConfig,

    // Main stored data
    cache: NodeStatusCache,

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
    pub(crate) fn new(
        cache: NodeStatusCache,
        config: NodeStatusCacheConfig,
        contract_cache: MixnetContractCache,
        described_cache: SharedCache<DescribedNodes>,
        contract_cache_listener: CacheNotificationWatcher,
        describe_cache_listener: CacheNotificationWatcher,
        performance_provider: Box<dyn NodePerformanceProvider + Send + Sync>,
        on_disk_file: PathBuf,
    ) -> Self {
        Self {
            cache,
            config,
            mixnet_contract_cache: contract_cache,
            described_cache,
            mixnet_contract_cache_listener: contract_cache_listener,
            describe_cache_listener,
            refresh_requester: Default::default(),
            on_disk_file,
            performance_provider,
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
        &self,
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

    pub(crate) async fn produce_node_annotations(
        &self,
        config_score_data: &ConfigScoreData,
        routing_scores: &NodesRoutingScores,
        stress_testing_scores: &NodesStressTestingScores,
        nym_nodes: &[NymNodeDetails],
        rewarded_set: &CachedEpochRewardedSet,
        described_nodes: &DescribedNodes,
    ) -> HashMap<NodeId, NodeAnnotationV2> {
        let mut annotations = HashMap::new();
        if nym_nodes.is_empty() {
            return annotations;
        }

        let use_stress_testing_scores = self.config.use_stress_testing_data;
        let available_ratio =
            stress_testing_scores.available_count() as f32 / nym_nodes.len() as f32;

        // must be explicitly enabled in the config AND we must have sufficient number of entries
        let include_stress_testing = use_stress_testing_scores
            && available_ratio >= self.config.minimum_available_stress_testing_results;

        // stress testing
        let sw = self.config.stress_testing_score_weight;

        // not stress testing
        let nsw = 1.0 - sw;

        for nym_node in nym_nodes {
            let node_id = nym_node.node_id();
            let routing_score = routing_scores.get_or_log(node_id);
            let config_score =
                calculate_config_score(config_score_data, described_nodes.get_node(&node_id));
            let stress_testing_score = stress_testing_scores.get_or_log(node_id);

            let performance = if include_stress_testing {
                // use weighted arithmetic mean (we don't want a single 0 to cause the whole thing to be 0)
                sw * stress_testing_score.score + nsw * routing_score.score * config_score.score
            } else {
                info!("not using stress testing data for performance calculation");
                routing_score.score * config_score.score
            };

            annotations.insert(
                nym_node.node_id(),
                NodeAnnotationV2 {
                    current_role: rewarded_set.role(nym_node.node_id()).map(|r| r.into()),
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

    /// Refreshes the node status cache by fetching the latest data from the contract cache
    #[allow(deprecated)]
    async fn refresh(&self) -> Result<(), NodeStatusCacheError> {
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

        // Create annotated data
        let node_annotations = self
            .produce_node_annotations(
                &config_score_data,
                &routing_scores,
                &stress_testing_scores,
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
