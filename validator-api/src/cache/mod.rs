// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd_client::Client;
use anyhow::Result;
use config::defaults::VALIDATOR_API_VERSION;
use mixnet_contract::{GatewayBond, MixNodeBond, RewardingIntervalResponse, StateParams};
use rand::prelude::SliceRandom;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rocket::fairing::AdHoc;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time;
use validator_client::nymd::hash::SHA256_HASH_SIZE;
use validator_client::nymd::CosmWasmClient;

pub(crate) mod routes;

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
}

#[derive(Default, Serialize, Clone)]
pub struct Cache<T> {
    value: T,
    as_at: u64,
}

impl<T: Clone> Cache<T> {
    fn set(&mut self, value: T) {
        self.value = value;
        self.as_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn renew(&mut self) {
        self.as_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
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

        let state_params = self.nymd_client.get_state_params().await?;
        let current_rewarding_interval = self.nymd_client.get_current_rewarding_interval().await?;
        let rewarding_block_hash = self
            .nymd_client
            .get_block_hash(
                current_rewarding_interval.current_rewarding_interval_starting_block as u32,
            )
            .await?;

        info!(
            "Updating validator cache. There are {} mixnodes and {} gateways",
            mixnodes.len(),
            gateways.len(),
        );

        self.cache
            .update_cache(
                mixnodes,
                gateways,
                state_params,
                current_rewarding_interval,
                rewarding_block_hash,
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

        // generate list of mixnodes and their relatively weight (by total stake)
        let choices = mixnodes
            .iter()
            .map(|mix| {
                // note that the theoretical maximum possible stake is equal to the total
                // supply of all tokens, i.e. 1B (which is 1 quadrillion of native tokens, i.e. 10^15 ~ 2^50)
                // which is way below maximum value of f64, so the cast is fine
                let total_stake = mix.total_stake().unwrap_or_default() as f64;
                (mix, total_stake)
            }) // if for some reason node is invalid, treat it as 0 stake/weight
            .collect::<Vec<_>>();

        // the unwrap here is fine as an error can only be thrown under one of the following conditions:
        // - our mixnode list is empty - we have already checked for that
        // - we have invalid weights, i.e. less than zero or NaNs - it shouldn't happen in our case as we safely cast down from u128
        // - all weights are zero - it's impossible in our case as the list of nodes is not empty and weight is proportional to stake. You must have non-zero stake in order to bond
        // - we have more than u32::MAX values (which is incredibly unrealistic to have 4B mixnodes bonded... literally every other person on the planet would need one)
        choices
            .choose_multiple_weighted(&mut rng, nodes_to_select as usize, |item| item.1)
            .unwrap()
            .map(|(bond, _weight)| *bond)
            .cloned()
            .collect()
    }

    async fn update_cache(
        &self,
        mixnodes: Vec<MixNodeBond>,
        gateways: Vec<GatewayBond>,
        state: StateParams,
        rewarding_interval: RewardingIntervalResponse,
        rewarding_block_hash: Option<[u8; SHA256_HASH_SIZE]>,
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
        }
    }
}
