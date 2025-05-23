// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NodeStatusCache;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_status_api::cache::node_sets::produce_node_annotations;
use crate::support::caching::cache::SharedCache;
use crate::{
    node_status_api::cache::NodeStatusCacheError, nym_contract_cache::cache::NymContractCache,
    storage::NymApiStorage, support::caching::CacheNotification,
};
use ::time::OffsetDateTime;
use nym_task::TaskClient;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time;
use tracing::{info, trace, warn};

// Long running task responsible for keeping the node status cache up-to-date.
pub struct NodeStatusCacheRefresher {
    // Main stored data
    cache: NodeStatusCache,
    fallback_caching_interval: Duration,

    // Sources for when refreshing data
    contract_cache: NymContractCache,
    described_cache: SharedCache<DescribedNodes>,
    contract_cache_listener: watch::Receiver<CacheNotification>,
    describe_cache_listener: watch::Receiver<CacheNotification>,
    storage: NymApiStorage,
}

impl NodeStatusCacheRefresher {
    pub(crate) fn new(
        cache: NodeStatusCache,
        fallback_caching_interval: Duration,
        contract_cache: NymContractCache,
        described_cache: SharedCache<DescribedNodes>,
        contract_cache_listener: watch::Receiver<CacheNotification>,
        describe_cache_listener: watch::Receiver<CacheNotification>,
        storage: NymApiStorage,
    ) -> Self {
        Self {
            cache,
            fallback_caching_interval,
            contract_cache,
            described_cache,
            contract_cache_listener,
            describe_cache_listener,
            storage,
        }
    }

    /// Runs the node status cache refresher task.
    pub async fn run(&mut self, mut shutdown: TaskClient) {
        let mut last_update = OffsetDateTime::now_utc();
        let mut fallback_interval = time::interval(self.fallback_caching_interval);
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    trace!("NodeStatusCacheRefresher: Received shutdown");
                }
                // Update node status cache when the contract cache / describe cache is updated
                Ok(_) = self.contract_cache_listener.changed() => {
                    tokio::select! {
                        _ = self.maybe_refresh(&mut fallback_interval, &mut last_update) => (),
                        _ = shutdown.recv() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                        }
                    }
                }
                Ok(_) = self.describe_cache_listener.changed() => {
                    tokio::select! {
                        _ = self.maybe_refresh(&mut fallback_interval, &mut last_update) => (),
                        _ = shutdown.recv() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                        }
                    }
                }
                // ... however, if we don't receive any notifications we fall back to periodic
                // refreshes
                _ = fallback_interval.tick() => {
                    tokio::select! {
                        _ = self.maybe_refresh(&mut fallback_interval, &mut last_update) => (),
                        _ = shutdown.recv() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                        }
                    }
                }
            }
        }
        info!("NodeStatusCacheRefresher: Exiting");
    }

    fn caches_available(&self) -> bool {
        let contract_cache = *self.contract_cache_listener.borrow() != CacheNotification::Start;
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

    /// Refreshes the node status cache by fetching the latest data from the contract cache
    #[allow(deprecated)]
    async fn refresh(&self) -> Result<(), NodeStatusCacheError> {
        info!("Updating node status cache");

        // Fetch contract cache data to work with
        let mixnode_details = self.contract_cache.legacy_mixnodes_all().await;
        let interval_reward_params = self.contract_cache.interval_reward_params().await?;
        let current_interval = self.contract_cache.current_interval().await?;
        let rewarded_set = self.contract_cache.rewarded_set_owned().await?;
        let gateway_bonds = self.contract_cache.legacy_gateways_all().await;
        let nym_nodes = self.contract_cache.nym_nodes().await;
        let config_score_data = self.contract_cache.maybe_config_score_data().await?;

        // Compute inclusion probabilities
        // (all legacy mixnodes have 0% chance of being selected)
        let inclusion_probabilities = crate::node_status_api::cache::inclusion_probabilities::InclusionProbabilities::legacy_zero(&mixnode_details);

        let Ok(described) = self.described_cache.get().await else {
            return Err(NodeStatusCacheError::UnavailableDescribedCache);
        };

        let mut legacy_gateway_mapping = HashMap::new();
        for gateway in &gateway_bonds {
            legacy_gateway_mapping.insert(gateway.identity().clone(), gateway.node_id);
        }

        // Create annotated data
        let node_annotations = produce_node_annotations(
            &self.storage,
            &config_score_data,
            &mixnode_details,
            &gateway_bonds,
            &nym_nodes,
            &rewarded_set,
            current_interval,
            &described,
        )
        .await;

        let mixnodes_annotated =
            crate::node_status_api::cache::node_sets::annotate_legacy_mixnodes_nodes_with_details(
                &self.storage,
                mixnode_details,
                interval_reward_params,
                current_interval,
                &rewarded_set,
            )
            .await;

        let gateways_annotated =
            crate::node_status_api::cache::node_sets::annotate_legacy_gateways_with_details(
                &self.storage,
                gateway_bonds,
                current_interval,
            )
            .await;

        // Update the cache
        self.cache
            .update(
                legacy_gateway_mapping,
                node_annotations,
                mixnodes_annotated,
                gateways_annotated,
                inclusion_probabilities,
            )
            .await;
        Ok(())
    }
}
