use super::NodeStatusCache;
use crate::{
    contract_cache::cache::ValidatorCache,
    node_status_api::{
        cache::{
            helpers::{split_into_active_and_rewarded_set, to_rewarded_set_node_status},
            inclusion_probabilities::InclusionProbabilities,
            NodeStatusCacheError,
        },
        reward_estimate::{compute_apy_from_reward, compute_reward_estimate},
    },
    storage::NymApiStorage,
    support::caching::CacheNotification,
};
use mixnet_contract_common::{
    families::FamilyHead, reward_params::Performance, IdentityKey, Interval, MixId, MixNodeDetails,
    RewardedSetNodeStatus, RewardingParams,
};
use nym_api_requests::models::MixNodeBondAnnotated;
use std::{collections::HashMap, time::Duration};
use task::TaskClient;
use tokio::sync::watch;
use tokio::time;

// Long running task responsible of keeping the cache up-to-date.
pub struct NodeStatusCacheRefresher {
    // Main stored data
    cache: NodeStatusCache,
    fallback_caching_interval: Duration,

    // Sources for when refreshing data
    contract_cache: ValidatorCache,
    contract_cache_listener: watch::Receiver<CacheNotification>,
    storage: Option<NymApiStorage>,
}

impl NodeStatusCacheRefresher {
    pub(crate) fn new(
        cache: NodeStatusCache,
        fallback_caching_interval: Duration,
        contract_cache: ValidatorCache,
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

    async fn update_on_notify(&self, fallback_interval: &mut time::Interval) {
        log::debug!(
            "Validator cache event detected: {:?}",
            &*self.contract_cache_listener.borrow(),
        );
        let _ = self.refresh().await;
        fallback_interval.reset();
    }

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

    async fn refresh(&self) -> Result<(), NodeStatusCacheError> {
        log::info!("Updating node status cache");

        // Fetch contract cache data to work with
        let mixnode_details = self.contract_cache.mixnodes().await;
        let interval_reward_params = self.contract_cache.interval_reward_params().await;
        let current_interval = self.contract_cache.current_interval().await;

        let rewarded_set = self.contract_cache.rewarded_set().await;
        let active_set = self.contract_cache.active_set().await;
        let mix_to_family = self.contract_cache.mix_to_family().await;

        let interval_reward_params =
            interval_reward_params.ok_or(NodeStatusCacheError::SourceDataMissing)?;
        let current_interval = current_interval.ok_or(NodeStatusCacheError::SourceDataMissing)?;

        // Compute inclusion probabilities
        let inclusion_probabilities = InclusionProbabilities::compute(
            &mixnode_details,
            interval_reward_params,
        )
        .ok_or_else(|| {
            error!("Failed to simulate selection probabilties for mixnodes, not updating cache");
            NodeStatusCacheError::SimulationFailed
        })?;

        // Create annotated data
        let rewarded_set_node_status = to_rewarded_set_node_status(&rewarded_set, &active_set);
        let mixnodes_annotated = self
            .annotate_node_with_details(
                mixnode_details,
                interval_reward_params,
                current_interval,
                &rewarded_set_node_status,
                mix_to_family.to_vec(),
            )
            .await;

        // Create the annotated rewarded and active sets
        let (rewarded_set, active_set) =
            split_into_active_and_rewarded_set(&mixnodes_annotated, &rewarded_set_node_status);

        self.cache
            .update(
                mixnodes_annotated,
                rewarded_set,
                active_set,
                inclusion_probabilities,
            )
            .await;
        Ok(())
    }

    async fn get_performance_from_storage(
        &self,
        mix_id: MixId,
        epoch: Interval,
    ) -> Option<Performance> {
        self.storage
            .as_ref()?
            .get_average_mixnode_uptime_in_the_last_24hrs(
                mix_id,
                epoch.current_epoch_end_unix_timestamp(),
            )
            .await
            .ok()
            .map(Into::into)
    }

    async fn annotate_node_with_details(
        &self,
        mixnodes: Vec<MixNodeDetails>,
        interval_reward_params: RewardingParams,
        current_interval: Interval,
        rewarded_set: &HashMap<MixId, RewardedSetNodeStatus>,
        mix_to_family: Vec<(IdentityKey, FamilyHead)>,
    ) -> Vec<MixNodeBondAnnotated> {
        let mix_to_family = mix_to_family
            .into_iter()
            .collect::<HashMap<IdentityKey, FamilyHead>>();

        let mut annotated = Vec::new();
        for mixnode in mixnodes {
            let stake_saturation = mixnode
                .rewarding_details
                .bond_saturation(&interval_reward_params);

            let uncapped_stake_saturation = mixnode
                .rewarding_details
                .uncapped_bond_saturation(&interval_reward_params);

            // If the performance can't be obtained, because the nym-api was not started with
            // the monitoring (and hence, storage), then reward estimates will be all zero
            let performance = self
                .get_performance_from_storage(mixnode.mix_id(), current_interval)
                .await
                .unwrap_or_default();

            let rewarded_set_status = rewarded_set.get(&mixnode.mix_id()).copied();

            let reward_estimate = compute_reward_estimate(
                &mixnode,
                performance,
                rewarded_set_status,
                interval_reward_params,
                current_interval,
            );

            let (estimated_operator_apy, estimated_delegators_apy) =
                compute_apy_from_reward(&mixnode, reward_estimate, current_interval);

            let family = mix_to_family
                .get(&mixnode.bond_information.identity().to_string())
                .cloned();

            annotated.push(MixNodeBondAnnotated {
                mixnode_details: mixnode,
                stake_saturation,
                uncapped_stake_saturation,
                performance,
                estimated_operator_apy,
                estimated_delegators_apy,
                family,
            });
        }
        annotated
    }
}
