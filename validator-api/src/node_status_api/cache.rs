// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_cache::{Cache, CacheNotification, ValidatorCache};
use mixnet_contract_common::rewarding::helpers::truncate_decimal;
use mixnet_contract_common::{MixNodeDetails, NodeId, RewardingParams};
use rocket::fairing::AdHoc;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use tap::TapFallible;
use task::ShutdownListener;
use tokio::{
    sync::{watch, RwLock},
    time,
};
use validator_api_requests::models::InclusionProbability;

const CACHE_TIMOUT_MS: u64 = 100;
const MAX_SIMULATION_SAMPLES: u64 = 5000;
const MAX_SIMULATION_TIME_SEC: u64 = 15;

enum NodeStatusCacheError {
    SimulationFailed,
}

// A node status cache suitable for caching values computed in one sweep, such as active set
// inclusion probabilities that are computed for all mixnodes at the same time.
//
// The cache can be triggered to update on contract cache changes, and/or periodically on a timer.
#[derive(Clone)]
pub struct NodeStatusCache {
    inner: Arc<RwLock<NodeStatusCacheInner>>,
}

struct NodeStatusCacheInner {
    inclusion_probabilities: Cache<InclusionProbabilities>,
}

#[derive(Clone, Default, Serialize, schemars::JsonSchema)]
pub(crate) struct InclusionProbabilities {
    pub inclusion_probabilities: Vec<InclusionProbability>,
    pub samples: u64,
    pub elapsed: Duration,
    pub delta_max: f64,
    pub delta_l2: f64,
}

impl InclusionProbabilities {
    pub fn node(&self, mix_id: NodeId) -> Option<&InclusionProbability> {
        self.inclusion_probabilities.iter().find(|x| x.id == mix_id)
    }
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

    async fn update_cache(&self, inclusion_probabilities: InclusionProbabilities) {
        match time::timeout(Duration::from_millis(CACHE_TIMOUT_MS), self.inner.write()).await {
            Ok(mut cache) => {
                cache
                    .inclusion_probabilities
                    .update(inclusion_probabilities);
            }
            Err(e) => error!("{e}"),
        }
    }

    pub(crate) async fn inclusion_probabilities(&self) -> Option<Cache<InclusionProbabilities>> {
        match time::timeout(Duration::from_millis(CACHE_TIMOUT_MS), self.inner.read()).await {
            Ok(cache) => Some(cache.inclusion_probabilities.clone()),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }
}

impl NodeStatusCacheInner {
    pub fn new() -> Self {
        Self {
            inclusion_probabilities: Default::default(),
        }
    }
}

// Long running task responsible of keeping the cache up-to-date.
pub struct NodeStatusCacheRefresher {
    cache: NodeStatusCache,
    contract_cache: ValidatorCache,
    contract_cache_listener: watch::Receiver<CacheNotification>,
    fallback_caching_interval: Duration,
}

impl NodeStatusCacheRefresher {
    pub(crate) fn new(
        cache: NodeStatusCache,
        contract_cache: ValidatorCache,
        contract_cache_listener: watch::Receiver<CacheNotification>,
        fallback_caching_interval: Duration,
    ) -> Self {
        Self {
            cache,
            contract_cache,
            contract_cache_listener,
            fallback_caching_interval,
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
        let mixnode_bonds = self.contract_cache.mixnodes().await;
        let params = self
            .contract_cache
            .interval_reward_params()
            .await
            .into_inner()
            .ok_or(NodeStatusCacheError::SimulationFailed)?;
        let inclusion_probabilities = compute_inclusion_probabilities(&mixnode_bonds, params)
            .ok_or_else(|| {
                error!(
                    "Failed to simulate selection probabilties for mixnodes, not updating cache"
                );
                NodeStatusCacheError::SimulationFailed
            })?;

        self.cache.update_cache(inclusion_probabilities).await;
        Ok(())
    }
}

fn compute_inclusion_probabilities(
    mixnodes: &[MixNodeDetails],
    params: RewardingParams,
) -> Option<InclusionProbabilities> {
    let active_set_size = params.active_set_size;
    let standby_set_size = params.rewarded_set_size - active_set_size;

    // Unzip list of total bonds into ids and bonds.
    // We need to go through this zip/unzip procedure to make sure we have matching identities
    // for the input to the simulator, which assumes the identity is the position in the vec
    let (ids, mixnode_total_bonds) = unzip_into_mixnode_ids_and_total_bonds(mixnodes);

    // Compute inclusion probabilitites and keep track of how long time it took.
    let mut rng = rand::thread_rng();
    let results = inclusion_probability::simulate_selection_probability_mixnodes(
        &mixnode_total_bonds,
        active_set_size as usize,
        standby_set_size as usize,
        MAX_SIMULATION_SAMPLES,
        Duration::from_secs(MAX_SIMULATION_TIME_SEC),
        &mut rng,
    )
    .tap_err(|err| error!("{err}"))
    .ok()?;

    Some(InclusionProbabilities {
        inclusion_probabilities: zip_ids_together_with_results(&ids, &results),
        samples: results.samples,
        elapsed: results.time,
        delta_max: results.delta_max,
        delta_l2: results.delta_l2,
    })
}

fn unzip_into_mixnode_ids_and_total_bonds(mixnodes: &[MixNodeDetails]) -> (Vec<NodeId>, Vec<u128>) {
    mixnodes
        .iter()
        .map(|m| (m.mix_id(), truncate_decimal(m.total_stake()).u128()))
        .unzip()
}

fn zip_ids_together_with_results(
    ids: &[NodeId],
    results: &inclusion_probability::SelectionProbability,
) -> Vec<InclusionProbability> {
    ids.iter()
        .zip(results.active_set_probability.iter())
        .zip(results.reserve_set_probability.iter())
        .map(|((&id, a), r)| InclusionProbability {
            id,
            in_active: *a,
            in_reserve: *r,
        })
        .collect()
}
