// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_cache::{Cache, CacheNotification, ValidatorCache};
use crate::storage::ValidatorApiStorage;
use mixnet_contract_common::families::FamilyHead;
use mixnet_contract_common::reward_params::Performance;
use mixnet_contract_common::{
    IdentityKey, Interval, MixId, MixNodeDetails, RewardedSetNodeStatus, RewardingParams,
};
use nym_api_requests::models::{MixNodeBondAnnotated, MixnodeStatus};
use rocket::fairing::AdHoc;
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use task::ShutdownListener;
use tokio::sync::RwLockReadGuard;
use tokio::{
    sync::{watch, RwLock},
    time,
};

use self::inclusion_probabilities::InclusionProbabilities;

use super::reward_estimate::{compute_apy_from_reward, compute_reward_estimate};

mod inclusion_probabilities;

const CACHE_TIMOUT_MS: u64 = 100;

enum NodeStatusCacheError {
    SimulationFailed,
    SourceDataMissing,
}

// A node status cache suitable for caching values computed in one sweep, such as active set
// inclusion probabilities that are computed for all mixnodes at the same time.
//
// The cache can be triggered to update on contract cache changes, and/or periodically on a timer.
#[derive(Clone)]
pub struct NodeStatusCache {
    inner: Arc<RwLock<NodeStatusCacheInner>>,
}

#[derive(Default)]
struct NodeStatusCacheInner {
    mixnodes_annotated: Cache<Vec<MixNodeBondAnnotated>>,
    rewarded_set_annotated: Cache<Vec<MixNodeBondAnnotated>>,
    active_set_annotated: Cache<Vec<MixNodeBondAnnotated>>,

    // Estimated active set inclusion probabilities from Monte Carlo simulation
    inclusion_probabilities: Cache<InclusionProbabilities>,
}

impl NodeStatusCache {
    fn new() -> Self {
        NodeStatusCache {
            inner: Arc::new(RwLock::new(NodeStatusCacheInner::new())),
        }
    }

    pub fn stage() -> AdHoc {
        AdHoc::on_ignite("Node Status Cache", |rocket| async {
            rocket.manage(Self::new())
        })
    }

    async fn update_cache(
        &self,
        mixnodes: Vec<MixNodeBondAnnotated>,
        rewarded_set: Vec<MixNodeBondAnnotated>,
        active_set: Vec<MixNodeBondAnnotated>,
        inclusion_probabilities: InclusionProbabilities,
    ) {
        match time::timeout(Duration::from_millis(CACHE_TIMOUT_MS), self.inner.write()).await {
            Ok(mut cache) => {
                cache.mixnodes_annotated.update(mixnodes);
                cache.rewarded_set_annotated.update(rewarded_set);
                cache.active_set_annotated.update(active_set);
                cache
                    .inclusion_probabilities
                    .update(inclusion_probabilities);
            }
            Err(e) => error!("{e}"),
        }
    }

    async fn get_cache<T>(
        &self,
        fn_arg: impl FnOnce(RwLockReadGuard<'_, NodeStatusCacheInner>) -> Cache<T>,
    ) -> Option<Cache<T>> {
        match time::timeout(Duration::from_millis(CACHE_TIMOUT_MS), self.inner.read()).await {
            Ok(cache) => Some(fn_arg(cache)),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }

    pub(crate) async fn mixnodes_annotated(&self) -> Option<Cache<Vec<MixNodeBondAnnotated>>> {
        self.get_cache(|c| c.mixnodes_annotated.clone()).await
    }

    pub(crate) async fn rewarded_set_annotated(&self) -> Option<Cache<Vec<MixNodeBondAnnotated>>> {
        self.get_cache(|c| c.rewarded_set_annotated.clone()).await
    }

    pub(crate) async fn active_set_annotated(&self) -> Option<Cache<Vec<MixNodeBondAnnotated>>> {
        self.get_cache(|c| c.active_set_annotated.clone()).await
    }

    pub(crate) async fn inclusion_probabilities(&self) -> Option<Cache<InclusionProbabilities>> {
        self.get_cache(|c| c.inclusion_probabilities.clone()).await
    }

    pub async fn mixnode_details(
        &self,
        mix_id: MixId,
    ) -> (Option<MixNodeBondAnnotated>, MixnodeStatus) {
        // it might not be the most optimal to possibly iterate the entire vector to find (or not)
        // the relevant value. However, the vectors are relatively small (< 10_000 elements, < 1000 for active set)

        let active_set = &self.active_set_annotated().await.unwrap().into_inner();
        if let Some(bond) = active_set.iter().find(|mix| mix.mix_id() == mix_id) {
            return (Some(bond.clone()), MixnodeStatus::Active);
        }

        let rewarded_set = &self.rewarded_set_annotated().await.unwrap().into_inner();
        if let Some(bond) = rewarded_set.iter().find(|mix| mix.mix_id() == mix_id) {
            return (Some(bond.clone()), MixnodeStatus::Standby);
        }

        let all_bonded = &self.mixnodes_annotated().await.unwrap().into_inner();
        if let Some(bond) = all_bonded.iter().find(|mix| mix.mix_id() == mix_id) {
            (Some(bond.clone()), MixnodeStatus::Inactive)
        } else {
            (None, MixnodeStatus::NotFound)
        }
    }
}

impl NodeStatusCacheInner {
    pub fn new() -> Self {
        Self::default()
    }
}

// Long running task responsible of keeping the cache up-to-date.
pub struct NodeStatusCacheRefresher {
    // Main stored data
    cache: NodeStatusCache,
    fallback_caching_interval: Duration,

    // Sources for when refreshing data
    contract_cache: ValidatorCache,
    contract_cache_listener: watch::Receiver<CacheNotification>,
    storage: Option<ValidatorApiStorage>,
}

impl NodeStatusCacheRefresher {
    pub(crate) fn new(
        cache: NodeStatusCache,
        fallback_caching_interval: Duration,
        contract_cache: ValidatorCache,
        contract_cache_listener: watch::Receiver<CacheNotification>,
        storage: Option<ValidatorApiStorage>,
    ) -> Self {
        Self {
            cache,
            fallback_caching_interval,
            contract_cache,
            contract_cache_listener,
            storage,
        }
    }

    pub async fn run(&mut self, mut shutdown: ShutdownListener) {
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
        let _ = self.refresh_cache().await;
        fallback_interval.reset();
    }

    async fn update_on_timer(&self) {
        log::debug!("Timed trigger for the node status cache");
        let have_contract_cache_data =
            *self.contract_cache_listener.borrow() != CacheNotification::Start;

        if have_contract_cache_data {
            let _ = self.refresh_cache().await;
        } else {
            log::trace!(
                "Skipping updating node status cache, is the contract cache not yet available?"
            );
        }
    }

    async fn refresh_cache(&self) -> Result<(), NodeStatusCacheError> {
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
            .update_cache(
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

fn to_rewarded_set_node_status(
    rewarded_set: &[MixNodeDetails],
    active_set: &[MixNodeDetails],
) -> HashMap<MixId, RewardedSetNodeStatus> {
    let mut rewarded_set_node_status: HashMap<MixId, RewardedSetNodeStatus> = rewarded_set
        .iter()
        .map(|m| (m.mix_id(), RewardedSetNodeStatus::Standby))
        .collect();
    for mixnode in active_set {
        *rewarded_set_node_status
            .get_mut(&mixnode.mix_id())
            .expect("All active nodes are rewarded nodes") = RewardedSetNodeStatus::Active;
    }
    rewarded_set_node_status
}

fn split_into_active_and_rewarded_set(
    mixnodes_annotated: &[MixNodeBondAnnotated],
    rewarded_set_node_status: &HashMap<u32, RewardedSetNodeStatus>,
) -> (Vec<MixNodeBondAnnotated>, Vec<MixNodeBondAnnotated>) {
    let rewarded_set: Vec<_> = mixnodes_annotated
        .iter()
        .filter(|mixnode| rewarded_set_node_status.get(&mixnode.mix_id()).is_some())
        .cloned()
        .collect();
    let active_set: Vec<_> = rewarded_set
        .iter()
        .filter(|mixnode| {
            rewarded_set_node_status
                .get(&mixnode.mix_id())
                .map_or(false, RewardedSetNodeStatus::is_active)
        })
        .cloned()
        .collect();
    (rewarded_set, active_set)
}
