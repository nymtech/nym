// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NodeStatusCache;
use crate::{
    node_status_api::cache::{
        inclusion_probabilities::InclusionProbabilities,
        node_sets::{
            annotate_gateways_with_details, annotate_nodes_with_details,
            split_into_active_and_rewarded_set, to_rewarded_set_node_status,
        },
        NodeStatusCacheError,
    },
    nym_contract_cache::cache::NymContractCache,
    storage::NymApiStorage,
    support::caching::CacheNotification,
};
use log::error;
use nym_task::TaskClient;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time;

// Long running task responsible for keeping the node status cache up-to-date.
pub struct NodeStatusCacheRefresher {
    // Main stored data
    cache: NodeStatusCache,
    fallback_caching_interval: Duration,

    // Sources for when refreshing data
    contract_cache: NymContractCache,
    contract_cache_listener: watch::Receiver<CacheNotification>,
    storage: Option<NymApiStorage>,
}

impl NodeStatusCacheRefresher {
    pub(crate) fn new(
        cache: NodeStatusCache,
        fallback_caching_interval: Duration,
        contract_cache: NymContractCache,
        contract_cache_listener: watch::Receiver<CacheNotification>,
        storage: Option<NymApiStorage>,
    ) -> Self {
        Self {
            cache,
            fallback_caching_interval,
            contract_cache,
            contract_cache_listener,
            storage,
        }
    }

    /// Runs the node status cache refresher task.
    pub async fn run(&mut self, mut shutdown: TaskClient) {
        let mut fallback_interval = time::interval(self.fallback_caching_interval);
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("NodeStatusCacheRefresher: Received shutdown");
                }
                // Update node status cache when the contract cache / validator cache is updated
                Ok(_) = self.contract_cache_listener.changed() => {
                    tokio::select! {
                        _ = self.update_on_notify(&mut fallback_interval) => (),
                        _ = shutdown.recv() => {
                            log::trace!("NodeStatusCacheRefresher: Received shutdown");
                        }
                    }
                }
                // ... however, if we don't receive any notifications we fall back to periodic
                // refreshes
                _ = fallback_interval.tick() => {
                    tokio::select! {
                        _ = self.update_on_timer() => (),
                        _ = shutdown.recv() => {
                            log::trace!("NodeStatusCacheRefresher: Received shutdown");
                        }
                    }
                }
            }
        }
        log::info!("NodeStatusCacheRefresher: Exiting");
    }

    /// Updates the node status cache when the contract cache / validator cache is updated
    async fn update_on_notify(&self, fallback_interval: &mut time::Interval) {
        log::debug!(
            "Validator cache event detected: {:?}",
            &*self.contract_cache_listener.borrow(),
        );
        let _ = self.refresh().await;
        fallback_interval.reset();
    }

    /// Updates the node status cache when the fallback interval is reached
    async fn update_on_timer(&self) {
        log::debug!("Timed trigger for the node status cache");
        let have_contract_cache_data =
            *self.contract_cache_listener.borrow() != CacheNotification::Start;

        if have_contract_cache_data {
            let _ = self.refresh().await;
        } else {
            log::trace!(
                "Skipping updating node status cache, is the contract cache not yet available?"
            );
        }
    }

    /// Refreshes the node status cache by fetching the latest data from the contract cache
    async fn refresh(&self) -> Result<(), NodeStatusCacheError> {
        log::info!("Updating node status cache");

        // Fetch contract cache data to work with
        let mixnode_details = self.contract_cache.mixnodes_all().await;
        let interval_reward_params = self.contract_cache.interval_reward_params().await;
        let current_interval = self.contract_cache.current_interval().await;
        let rewarded_set = self.contract_cache.rewarded_set().await;
        let active_set = self.contract_cache.active_set().await;
        let mix_to_family = self.contract_cache.mix_to_family().await;
        let gateway_bonds = self.contract_cache.gateways_all().await;

        // get blacklists
        let mixnodes_blacklist = self.contract_cache.mixnodes_blacklist().await;
        let gateways_blacklist = self.contract_cache.gateways_blacklist().await;

        let interval_reward_params =
            interval_reward_params.ok_or(NodeStatusCacheError::SourceDataMissing)?;
        let current_interval = current_interval.ok_or(NodeStatusCacheError::SourceDataMissing)?;

        // Compute inclusion probabilities
        let inclusion_probabilities = InclusionProbabilities::compute(
            &mixnode_details,
            interval_reward_params,
        )
        .ok_or_else(|| {
            error!("Failed to simulate selection probabilities for mixnodes, not updating cache");
            NodeStatusCacheError::SimulationFailed
        })?;

        // Create annotated data
        let rewarded_set_node_status = to_rewarded_set_node_status(&rewarded_set, &active_set);
        let mixnodes_annotated = annotate_nodes_with_details(
            &self.storage,
            mixnode_details,
            interval_reward_params,
            current_interval,
            &rewarded_set_node_status,
            mix_to_family.to_vec(),
            &mixnodes_blacklist,
        )
        .await;

        // Create the annotated rewarded and active sets
        let (rewarded_set, active_set) =
            split_into_active_and_rewarded_set(&mixnodes_annotated, &rewarded_set_node_status);

        let gateways_annotated = annotate_gateways_with_details(
            &self.storage,
            gateway_bonds,
            current_interval,
            &gateways_blacklist,
        )
        .await;

        // Update the cache
        self.cache
            .update(
                mixnodes_annotated,
                rewarded_set,
                active_set,
                gateways_annotated,
                inclusion_probabilities,
            )
            .await;
        Ok(())
    }
}
