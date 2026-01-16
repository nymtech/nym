// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NodeStatusCache;
use crate::mixnet_contract_cache::cache::data::ConfigScoreData;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_performance::provider::{NodePerformanceProvider, NodesRoutingScores};
use crate::node_status_api::cache::config_score::calculate_config_score;
use crate::node_status_api::models::Uptime;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::RefreshRequester;
use crate::{
    mixnet_contract_cache::cache::MixnetContractCache,
    node_status_api::cache::NodeStatusCacheError, support::caching::CacheNotification,
};
use ::time::OffsetDateTime;
use nym_api_requests::models::{DetailedNodePerformance, NodeAnnotation};
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_task::ShutdownToken;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time;
use tracing::{error, info, trace, warn};

// Long running task responsible for keeping the node status cache up-to-date.
pub struct NodeStatusCacheRefresher {
    // Main stored data
    cache: NodeStatusCache,
    fallback_caching_interval: Duration,

    // Sources for when refreshing data
    mixnet_contract_cache: MixnetContractCache,
    described_cache: SharedCache<DescribedNodes>,

    /// channel notifying us when mixnet cache has been refreshed,
    /// so that this cache could also be recreated
    mixnet_contract_cache_listener: watch::Receiver<CacheNotification>,

    /// channel notifying us when the describe cache has been refreshed,
    /// so that this cache could also be recreated
    describe_cache_listener: watch::Receiver<CacheNotification>,

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
        fallback_caching_interval: Duration,
        contract_cache: MixnetContractCache,
        described_cache: SharedCache<DescribedNodes>,
        contract_cache_listener: watch::Receiver<CacheNotification>,
        describe_cache_listener: watch::Receiver<CacheNotification>,
        performance_provider: Box<dyn NodePerformanceProvider + Send + Sync>,
        on_disk_file: PathBuf,
    ) -> Self {
        Self {
            cache,
            fallback_caching_interval,
            mixnet_contract_cache: contract_cache,
            described_cache,
            mixnet_contract_cache_listener: contract_cache_listener,
            describe_cache_listener,
            refresh_requester: Default::default(),
            on_disk_file,
            performance_provider,
        }
    }

    /// Runs the node status cache refresher task.
    pub async fn run(&mut self, shutdown_token: ShutdownToken) {
        let mut last_update = OffsetDateTime::now_utc();
        let mut fallback_interval = time::interval(self.fallback_caching_interval);
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

        if OffsetDateTime::now_utc() - *last_updated < self.fallback_caching_interval {
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
        nym_nodes: &[NymNodeDetails],
        rewarded_set: &CachedEpochRewardedSet,
        described_nodes: &DescribedNodes,
    ) -> HashMap<NodeId, NodeAnnotation> {
        let mut annotations = HashMap::new();

        for nym_node in nym_nodes {
            let node_id = nym_node.node_id();
            let routing_score = routing_scores.get_or_log(node_id);
            let config_score =
                calculate_config_score(config_score_data, described_nodes.get_node(&node_id));

            let performance = routing_score.score * config_score.score;
            // map it from 0-1 range into 0-100
            let scaled_performance = performance * 100.;
            let legacy_performance = Uptime::new(scaled_performance as f32).into();

            annotations.insert(
                nym_node.node_id(),
                NodeAnnotation {
                    last_24h_performance: legacy_performance,
                    current_role: rewarded_set.role(nym_node.node_id()).map(|r| r.into()),
                    detailed_performance: DetailedNodePerformance::new(
                        performance,
                        routing_score,
                        config_score,
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
            .collect();

        // note: any internal errors imply failures for that node in particular
        let routing_scores = self
            .performance_provider
            .get_batch_node_scores(all_ids, current_interval.current_epoch_absolute_id())
            .await?;

        // Create annotated data
        let node_annotations = self
            .produce_node_annotations(
                &config_score_data,
                &routing_scores,
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
