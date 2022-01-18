// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd_client::Client;
use crate::rewarding::EpochRewardParams;
use ::time::OffsetDateTime;
use anyhow::Result;
use config::defaults::VALIDATOR_API_VERSION;
use mixnet_contract_common::{
    ContractStateParams, GatewayBond, IdentityKey, IdentityKeyRef, MixNodeBond,
    RewardingIntervalResponse,
};
use rand::prelude::SliceRandom;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rocket::fairing::AdHoc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use validator_api_requests::models::MixnodeStatus;
use validator_client::nymd::hash::SHA256_HASH_SIZE;
use validator_client::nymd::CosmWasmClient;

pub(crate) mod routes;

#[derive(Debug, Serialize, Deserialize)]
pub struct InclusionProbabilityResponse {
    in_active: f32,
    in_reserve: f32,
}

impl fmt::Display for InclusionProbabilityResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "in_active: {:.5}, in_reserve: {:.5}",
            self.in_active, self.in_reserve
        )
    }
}

pub struct ValidatorCacheRefresher<C> {
    nymd_client: Client<C>,
    cache: ValidatorCache,
    caching_interval: Duration,
}

#[derive(Clone)]
pub struct ValidatorCache {
    inner: Arc<ValidatorCacheInner>,
}

struct ValidatorCacheInner {
    initialised: AtomicBool,
    latest_known_rewarding_block: AtomicU64,

    mixnodes: RwLock<Cache<Vec<MixNodeBond>>>,
    gateways: RwLock<Cache<Vec<GatewayBond>>>,

    rewarded_mixnodes: RwLock<Cache<Vec<MixNodeBond>>>,

    current_mixnode_rewarded_set_size: AtomicU32,
    current_mixnode_active_set_size: AtomicU32,

    current_reward_params: RwLock<Cache<EpochRewardParams>>,
}

fn current_unix_timestamp() -> i64 {
    let now = OffsetDateTime::now_utc();
    now.unix_timestamp()
}

#[derive(Default, Serialize, Clone)]
pub struct Cache<T> {
    value: T,
    as_at: i64,
}

impl<T: Clone> Cache<T> {
    fn new(value: T) -> Self {
        Cache {
            value,
            as_at: current_unix_timestamp(),
        }
    }

    fn set(&mut self, value: T) {
        self.value = value;
        self.as_at = current_unix_timestamp()
    }

    fn renew(&mut self) {
        self.as_at = current_unix_timestamp()
    }

    pub fn timestamp(&self) -> i64 {
        self.as_at
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<C> ValidatorCacheRefresher<C> {
    pub(crate) fn new(
        nymd_client: Client<C>,
        caching_interval: Duration,
        cache: ValidatorCache,
    ) -> Self {
        ValidatorCacheRefresher {
            nymd_client,
            cache,
            caching_interval,
        }
    }

    async fn refresh_cache(&self) -> Result<()>
    where
        C: CosmWasmClient + Sync,
    {
        let (mixnodes, gateways) = tokio::try_join!(
            self.nymd_client.get_mixnodes(),
            self.nymd_client.get_gateways(),
        )?;

        let contract_settings = self.nymd_client.get_contract_settings().await?;
        let current_rewarding_interval = self.nymd_client.get_current_rewarding_interval().await?;
        let rewarding_block_hash = self
            .nymd_client
            .get_block_hash(
                current_rewarding_interval.current_rewarding_interval_starting_block as u32,
            )
            .await?;

        let epoch_rewarding_params = self.nymd_client.get_current_epoch_reward_params().await?;

        info!(
            "Updating validator cache. There are {} mixnodes and {} gateways",
            mixnodes.len(),
            gateways.len(),
        );

        self.cache
            .update_cache(
                mixnodes,
                gateways,
                contract_settings,
                current_rewarding_interval,
                rewarding_block_hash,
                epoch_rewarding_params,
            )
            .await;

        Ok(())
    }

    pub(crate) async fn run(&self)
    where
        C: CosmWasmClient + Sync,
    {
        let mut interval = time::interval(self.caching_interval);
        loop {
            interval.tick().await;
            if let Err(err) = self.refresh_cache().await {
                error!("Failed to refresh validator cache - {}", err);
            } else {
                // relaxed memory ordering is fine here. worst case scenario network monitor
                // will just have to wait for an additional backoff to see the change.
                // And so this will not really incur any performance penalties by setting it every loop iteration
                self.cache.inner.initialised.store(true, Ordering::Relaxed)
            }
        }
    }
}

impl ValidatorCache {
    fn new() -> Self {
        ValidatorCache {
            inner: Arc::new(ValidatorCacheInner::new()),
        }
    }

    pub fn stage() -> AdHoc {
        AdHoc::on_ignite("Validator Cache Stage", |rocket| async {
            rocket.manage(Self::new()).mount(
                // this format! is so ugly...
                format!("/{}", VALIDATOR_API_VERSION),
                routes![
                    routes::get_mixnodes,
                    routes::get_gateways,
                    routes::get_active_mixnodes,
                    routes::get_rewarded_mixnodes,
                    routes::get_probs_mixnode_rewarded
                ],
            )
        })
    }

    // NOTE: this does not guarantee consistent results between multiple validator APIs, because
    // currently we do not guarantee the list of mixnodes (i.e. `mixnodes: &[MixNodeBond]`) will be the same -
    // somebody might bond/unbond a node or change delegation between different cache refreshes.
    //
    // I guess that's not a problem right now and we can resolve it later. My idea for that would be as follows:
    // since the demanded set changes only monthly, just write the identities of those nodes to the smart
    // contract upon finished rewarding (this works under assumption of rewards being distributed by a single validator)
    //
    // alternatively we could have some state locking mechanism for the duration of determining the demanded set
    // this could work with multiple validators via some multisig mechanism
    fn determine_rewarded_set(
        &self,
        mixnodes: &[MixNodeBond],
        nodes_to_select: u32,
        block_hash: Option<[u8; SHA256_HASH_SIZE]>,
    ) -> Vec<MixNodeBond> {
        if mixnodes.is_empty() {
            return Vec::new();
        }

        if block_hash.is_none() {
            // I'm not entirely sure under what condition can hash of a block be empty
            // (note that we know the block exists otherwise we would have gotten an error
            // when attempting to retrieve the hash)
            error!("The hash of the block of the rewarding interval is None - we're not going to update the set");
            return Vec::new();
        }

        // seed our rng with the hash of the block of the most recent rewarding interval
        let mut rng = ChaCha20Rng::from_seed(block_hash.unwrap());

        self.stake_weighted_choice(mixnodes, nodes_to_select as usize, &mut rng)
    }

    fn stake_weighted_choice(
        &self,
        mixnodes: &[MixNodeBond],
        nodes_to_select: usize,
        rng: &mut ChaCha20Rng,
    ) -> Vec<MixNodeBond> {
        // generate list of mixnodes and their relatively weight (by total stake)
        let choices = self.generate_mixnode_stake_tuples(mixnodes);
        // the unwrap here is fine as an error can only be thrown under one of the following conditions:
        // - our mixnode list is empty - we have already checked for that
        // - we have invalid weights, i.e. less than zero or NaNs - it shouldn't happen in our case as we safely cast down from u128
        // - all weights are zero - it's impossible in our case as the list of nodes is not empty and weight is proportional to stake. You must have non-zero stake in order to bond
        // - we have more than u32::MAX values (which is incredibly unrealistic to have 4B mixnodes bonded... literally every other person on the planet would need one)
        choices
            .choose_multiple_weighted(rng, nodes_to_select, |item| item.1)
            .unwrap()
            .map(|(bond, _weight)| bond)
            .cloned()
            .collect()
    }

    fn generate_mixnode_stake_tuples(&self, mixnodes: &[MixNodeBond]) -> Vec<(MixNodeBond, f64)> {
        mixnodes
            .iter()
            .map(|mix| {
                // note that the theoretical maximum possible stake is equal to the total
                // supply of all tokens, i.e. 1B (which is 1 quadrillion of native tokens, i.e. 10^15 ~ 2^50)
                // which is way below maximum value of f64, so the cast is fine
                let total_stake = mix.total_stake().unwrap_or_default() as f64;
                (mix.clone(), total_stake)
            }) // if for some reason node is invalid, treat it as 0 stake/weight
            .collect()
    }

    // Estimate probability that a node will end up in the rewarded set, by running the selection process 100 times and aggregating the results.
    // If a node is in the active set it is not counted as being in the reserve set, the probabilities are exclusive. Cumulative probabilitiy
    // can be obtained by summing the two probabilities returned.
    async fn probs_mixnode_rewarded_calculate(
        &self,
        target_mixnode_id: IdentityKey,
        mixnodes: Option<Vec<MixNodeBond>>,
    ) -> Option<InclusionProbabilityResponse> {
        let mixnodes = if let Some(nodes) = mixnodes {
            nodes
        } else {
            self.inner.mixnodes.read().await.value.clone()
        };
        let total_bonded_tokens = mixnodes
            .iter()
            .fold(0u128, |acc, x| acc + x.total_stake().unwrap_or_default())
            as f64;
        let target_mixnode = mixnodes
            .iter()
            .find(|x| x.identity() == &target_mixnode_id)?;
        let rewarded_set_size = self
            .inner
            .current_mixnode_rewarded_set_size
            .load(Ordering::SeqCst) as f64;
        let active_set_size = self
            .inner
            .current_mixnode_active_set_size
            .load(Ordering::SeqCst) as f64;

        // For running comparison tests below, needs improvement
        // let rewarded_set_size = 720.;
        // let active_set_size = 300.;

        let prob_one_draw =
            target_mixnode.total_stake().unwrap_or_default() as f64 / total_bonded_tokens;
        // Chance to be selected in any draw for active set
        let prob_active_set = active_set_size * prob_one_draw;
        // This is likely slightly too high, as we're not correcting form them not being selected in active, should be chance to be selected, minus the chance for being not selected in reserve
        let prob_reserve_set = (rewarded_set_size - active_set_size) * prob_one_draw;
        // (rewarded_set_size - active_set_size) * prob_one_draw * (1. - prob_active_set);

        Some(InclusionProbabilityResponse {
            in_active: if prob_active_set > 1. {
                1.
            } else {
                prob_active_set
            } as f32,
            in_reserve: if prob_reserve_set > 1. {
                1.
            } else {
                prob_reserve_set
            } as f32,
        })
    }

    #[allow(dead_code)]
    async fn probs_mixnode_rewarded_simulate(
        &self,
        target_mixnode_id: IdentityKey,
        mixnodes: Option<Vec<MixNodeBond>>,
    ) -> Option<InclusionProbabilityResponse> {
        let mut in_active = 0;
        let mut in_reserve = 0;
        let mixnodes = if let Some(nodes) = mixnodes {
            nodes
        } else {
            self.inner.mixnodes.read().await.value.clone()
        };
        let rewarded_set_size = self
            .inner
            .current_mixnode_rewarded_set_size
            .load(Ordering::SeqCst) as usize;
        let active_set_size = self
            .inner
            .current_mixnode_active_set_size
            .load(Ordering::SeqCst) as usize;
        let mut rng = ChaCha20Rng::from_entropy();

        // For running comparison tests below, needs improvement
        // let rewarded_set_size = 720.;
        // let active_set_size = 300.;

        let it = 100;

        for _ in 0..it {
            let mut rewarded_set =
                self.stake_weighted_choice(&mixnodes, rewarded_set_size, &mut rng);
            let reserve_set = rewarded_set
                .split_off(active_set_size)
                .iter()
                .map(|bond| bond.identity().clone())
                .collect::<HashSet<IdentityKey>>();
            let active_set = rewarded_set
                .iter()
                .map(|bond| bond.identity().clone())
                .collect::<HashSet<IdentityKey>>();

            if active_set.contains(&target_mixnode_id) {
                in_active += 1;
            } else if reserve_set.contains(&target_mixnode_id) {
                in_reserve += 1;
            }
        }
        Some(InclusionProbabilityResponse {
            in_active: in_active as f32 / it as f32,
            in_reserve: in_reserve as f32 / it as f32,
        })
    }

    async fn update_cache(
        &self,
        mixnodes: Vec<MixNodeBond>,
        gateways: Vec<GatewayBond>,
        state: ContractStateParams,
        rewarding_interval: RewardingIntervalResponse,
        rewarding_block_hash: Option<[u8; SHA256_HASH_SIZE]>,
        epoch_rewarding_params: EpochRewardParams,
    ) {
        // if the rewarding is currently in progress, don't mess with the rewarded/active sets
        // as most likely will be changed next time this function is called
        //
        // if our data is valid, it means the active sets are available,
        // otherwise we must explicitly indicate nobody can read this data
        if !rewarding_interval.rewarding_in_progress {
            // if we're still in the same rewarding interval, i.e. the latest block is the same,
            // there's nothing we have to do
            if rewarding_interval.current_rewarding_interval_starting_block
                > self
                    .inner
                    .latest_known_rewarding_block
                    .load(Ordering::SeqCst)
            {
                let rewarded_nodes = self.determine_rewarded_set(
                    &mixnodes,
                    state.mixnode_rewarded_set_size,
                    rewarding_block_hash,
                );

                self.inner
                    .current_mixnode_rewarded_set_size
                    .store(state.mixnode_rewarded_set_size, Ordering::SeqCst);
                self.inner
                    .current_mixnode_active_set_size
                    .store(state.mixnode_active_set_size, Ordering::SeqCst);
                self.inner.latest_known_rewarding_block.store(
                    rewarding_interval.current_rewarding_interval_starting_block,
                    Ordering::SeqCst,
                );

                self.inner
                    .rewarded_mixnodes
                    .write()
                    .await
                    .set(rewarded_nodes);
            } else {
                // however, update the timestamp on the cache
                self.inner.rewarded_mixnodes.write().await.renew()
            }
        }

        self.inner.mixnodes.write().await.set(mixnodes);
        self.inner.gateways.write().await.set(gateways);
        self.inner
            .current_reward_params
            .write()
            .await
            .set(epoch_rewarding_params);
    }

    pub async fn mixnodes(&self) -> Cache<Vec<MixNodeBond>> {
        self.inner.mixnodes.read().await.clone()
    }

    pub async fn gateways(&self) -> Cache<Vec<GatewayBond>> {
        self.inner.gateways.read().await.clone()
    }

    pub async fn rewarded_mixnodes(&self) -> Cache<Vec<MixNodeBond>> {
        self.inner.rewarded_mixnodes.read().await.clone()
    }

    pub async fn active_mixnodes(&self) -> Cache<Vec<MixNodeBond>> {
        // rewarded set is already "sorted" by pseudo-randomly choosing mixnodes from
        // all bonded nodes, weighted by stake. For the active set choose first k nodes.
        let cache = self.inner.rewarded_mixnodes.read().await;
        let timestamp = cache.as_at;
        let nodes = cache
            .value
            .iter()
            .take(
                self.inner
                    .current_mixnode_active_set_size
                    .load(Ordering::SeqCst) as usize,
            )
            .cloned()
            .collect();
        Cache {
            value: nodes,
            as_at: timestamp,
        }
    }

    pub(crate) async fn epoch_reward_params(&self) -> Cache<EpochRewardParams> {
        self.inner.current_reward_params.read().await.clone()
    }

    pub async fn mixnode_details(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> (Option<MixNodeBond>, MixnodeStatus) {
        // it might not be the most optimal to possibly iterate the entire vector to find (or not)
        // the relevant value. However, the vectors are relatively small (< 10_000 elements) and
        // the implementation for active/rewarded sets might change soon so there's no point in premature optimisation
        // with HashSets
        let rewarded_mixnodes = &self.inner.rewarded_mixnodes.read().await.value;
        let active_set_size = self
            .inner
            .current_mixnode_active_set_size
            .load(Ordering::SeqCst) as usize;

        // see if node is in the top active_set_size of rewarded nodes, i.e. it's active
        if let Some(bond) = rewarded_mixnodes
            .iter()
            .take(active_set_size)
            .find(|mix| mix.mix_node.identity_key == identity)
        {
            (Some(bond.clone()), MixnodeStatus::Active)
            // see if it's in the bottom part of the rewarded set, i.e. it's in standby
        } else if let Some(bond) = rewarded_mixnodes
            .iter()
            .skip(active_set_size)
            .find(|mix| mix.mix_node.identity_key == identity)
        {
            (Some(bond.clone()), MixnodeStatus::Standby)
            // if it's not in the rewarded set see if its bonded at all
        } else if let Some(bond) = self
            .inner
            .mixnodes
            .read()
            .await
            .value
            .iter()
            .find(|mix| mix.mix_node.identity_key == identity)
        {
            (Some(bond.clone()), MixnodeStatus::Inactive)
        } else {
            (None, MixnodeStatus::NotFound)
        }
    }

    pub async fn mixnode_status(&self, identity: IdentityKey) -> MixnodeStatus {
        self.mixnode_details(&identity).await.1
    }

    pub fn initialised(&self) -> bool {
        self.inner.initialised.load(Ordering::Relaxed)
    }

    pub(crate) async fn wait_for_initial_values(&self) {
        let initialisation_backoff = Duration::from_secs(5);
        loop {
            if self.initialised() {
                break;
            } else {
                debug!("Validator cache hasn't been initialised yet - waiting for {:?} before trying again", initialisation_backoff);
                tokio::time::sleep(initialisation_backoff).await;
            }
        }
    }
}

impl ValidatorCacheInner {
    fn new() -> Self {
        ValidatorCacheInner {
            initialised: AtomicBool::new(false),
            latest_known_rewarding_block: Default::default(),
            mixnodes: RwLock::new(Cache::default()),
            gateways: RwLock::new(Cache::default()),
            rewarded_mixnodes: RwLock::new(Cache::default()),
            current_mixnode_rewarded_set_size: Default::default(),
            current_mixnode_active_set_size: Default::default(),
            current_reward_params: RwLock::new(Cache::new(EpochRewardParams::new_empty())),
        }
    }
}

// #[cfg(test)]
// mod test {
//     use crate::cache::InclusionProbabilityResponse;

//     use super::ValidatorCache;
//     use mixnet_contract_common::MixNodeBond;

//     #[tokio::test]
//     async fn test_inclusion_probabilities() {
//         let cache = ValidatorCache::new();
//         let response = attohttpc::get("https://sandbox-validator.nymtech.net/api/v1/mixnodes")
//             .send()
//             .unwrap();
//         let mixnodes: Vec<MixNodeBond> = response.json().unwrap();
//         let calculated = cache
//             .probs_mixnode_rewarded_calculate(
//                 "ysmgeYJQPBFzB2TgqBjN5BE3Rb79CgFANrnJaYd8woQ".to_string(),
//                 Some(mixnodes.clone()),
//             )
//             .await
//             .unwrap();
//         let mut simulated_avg = Vec::new();
//         for _ in 0..1000 {
//             let simulated = cache
//                 .probs_mixnode_rewarded_simulate(
//                     "ysmgeYJQPBFzB2TgqBjN5BE3Rb79CgFANrnJaYd8woQ".to_string(),
//                     Some(mixnodes.clone()),
//                 )
//                 .await
//                 .unwrap();
//             simulated_avg.push(simulated);
//         }
//         let simulated = simulated_avg.iter().fold((0., 0.), |acc, x| {
//             (acc.0 + x.in_active, acc.1 + x.in_reserve)
//         });
//         let simulated = InclusionProbabilityResponse {
//             in_active: simulated.0 / 100.,
//             in_reserve: simulated.1 / 100.,
//         };
//         println!("calculted: {calculated}");
//         println!("simulated: {simulated}");
//     }
// }
