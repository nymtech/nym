// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd_client::Client;
use mixnet_contract_common::reward_params::EpochRewardParams;
use ::time::OffsetDateTime;
use anyhow::Result;
use config::defaults::VALIDATOR_API_VERSION;
use mixnet_contract_common::{
    GatewayBond, IdentityKey, IdentityKeyRef, Interval, MixNodeBond, RewardedSetNodeStatus,
};

use rocket::fairing::AdHoc;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time;
use validator_api_requests::models::MixnodeStatus;
use validator_client::nymd::CosmWasmClient;

pub(crate) mod routes;

type Epoch = Interval;

pub struct ValidatorCacheRefresher<C> {
    nymd_client: Client<C>,
    cache: ValidatorCache,
    caching_interval: Duration,
    update_rewarded_set_notify: Option<Arc<Notify>>,
}

#[derive(Clone)]
pub struct ValidatorCache {
    initialised: Arc<AtomicBool>,
    inner: Arc<RwLock<ValidatorCacheInner>>,
}

struct ValidatorCacheInner {
    mixnodes: Cache<Vec<MixNodeBond>>,
    gateways: Cache<Vec<GatewayBond>>,

    rewarded_set: Cache<Vec<MixNodeBond>>,
    active_set: Cache<Vec<MixNodeBond>>,

    current_reward_params: Cache<EpochRewardParams>,
    current_epoch: Cache<Interval>,
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

    fn update(&mut self, value: T) {
        self.value = value;
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
        update_rewarded_set_notify: Option<Arc<Notify>>,
    ) -> Self {
        ValidatorCacheRefresher {
            nymd_client,
            cache,
            caching_interval,
            update_rewarded_set_notify,
        }
    }

    fn collect_rewarded_and_active_set_details(
        &self,
        all_mixnodes: &[MixNodeBond],
        rewarded_set_identities: Vec<(IdentityKey, RewardedSetNodeStatus)>,
    ) -> (Vec<MixNodeBond>, Vec<MixNodeBond>) {
        let mut active_set = Vec::new();
        let mut rewarded_set = Vec::new();
        let rewarded_set_identities = rewarded_set_identities
            .into_iter()
            .collect::<HashMap<_, _>>();

        for mix in all_mixnodes {
            if let Some(status) = rewarded_set_identities.get(mix.identity()) {
                rewarded_set.push(mix.clone());
                if status.is_active() {
                    active_set.push(mix.clone())
                }
            }
        }

        (rewarded_set, active_set)
    }

    async fn refresh_cache(&self) -> Result<()>
    where
        C: CosmWasmClient + Sync,
    {
        let (mixnodes, gateways) = tokio::try_join!(
            self.nymd_client.get_mixnodes(),
            self.nymd_client.get_gateways(),
        )?;

        let rewarded_set_identities = self.nymd_client.get_rewarded_set_identities().await?;
        let (rewarded_set, active_set) =
            self.collect_rewarded_and_active_set_details(&mixnodes, rewarded_set_identities);

        let epoch_rewarding_params = self
            .nymd_client
            .get_current_epoch_reward_params()
            .await?;
        let current_interval = self.nymd_client.get_current_interval().await?;

        info!(
            "Updating validator cache. There are {} mixnodes and {} gateways",
            mixnodes.len(),
            gateways.len(),
        );

        self.cache
            .update_cache(
                mixnodes,
                gateways,
                rewarded_set,
                active_set,
                epoch_rewarding_params,
                current_interval,
            )
            .await;

        if let Some(notify) = &self.update_rewarded_set_notify {
            let update_details = self
                .nymd_client
                .get_current_rewarded_set_update_details()
                .await?;

            if update_details.last_refreshed_block + (update_details.refresh_rate_blocks as u64)
                < update_details.current_height
            {
                // there's only ever a single waiter -> the set updater
                notify.notify_one()
            }
        }

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
                self.cache.initialised.store(true, Ordering::Relaxed)
            }
        }
    }
}

impl ValidatorCache {
    fn new() -> Self {
        ValidatorCache {
            initialised: Arc::new(AtomicBool::new(false)),
            inner: Arc::new(RwLock::new(ValidatorCacheInner::new())),
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
                    routes::get_active_set,
                    routes::get_rewarded_set,
                ],
            )
        })
    }

    async fn update_cache(
        &self,
        mixnodes: Vec<MixNodeBond>,
        gateways: Vec<GatewayBond>,
        rewarded_set: Vec<MixNodeBond>,
        active_set: Vec<MixNodeBond>,
        epoch_rewarding_params: EpochRewardParams,
        current_epoch: Epoch,
    ) {
        let mut inner = self.inner.write().await;

        inner.mixnodes.update(mixnodes);
        inner.gateways.update(gateways);
        inner.rewarded_set.update(rewarded_set);
        inner.active_set.update(active_set);
        inner
            .current_reward_params
            .update(epoch_rewarding_params);
        inner.current_epoch.update(current_epoch);
    }

    pub async fn mixnodes(&self) -> Cache<Vec<MixNodeBond>> {
        self.inner.read().await.mixnodes.clone()
    }

    pub async fn gateways(&self) -> Cache<Vec<GatewayBond>> {
        self.inner.read().await.gateways.clone()
    }

    pub async fn rewarded_set(&self) -> Cache<Vec<MixNodeBond>> {
        self.inner.read().await.rewarded_set.clone()
    }

    pub async fn active_set(&self) -> Cache<Vec<MixNodeBond>> {
        self.inner.read().await.active_set.clone()
    }

    pub(crate) async fn epoch_reward_params(&self) -> Cache<EpochRewardParams> {
        self.inner.read().await.current_reward_params.clone()
    }

    pub(crate) async fn current_interval(&self) -> Cache<Interval> {
        self.inner.read().await.current_epoch.clone()
    }

    pub async fn mixnode_details(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> (Option<MixNodeBond>, MixnodeStatus) {
        // it might not be the most optimal to possibly iterate the entire vector to find (or not)
        // the relevant value. However, the vectors are relatively small (< 10_000 elements, < 1000 for active set)

        let active_set = &self.inner.read().await.active_set.value;
        if let Some(bond) = active_set
            .iter()
            .find(|mix| mix.mix_node.identity_key == identity)
        {
            return (Some(bond.clone()), MixnodeStatus::Active);
        }

        let rewarded_set = &self.inner.read().await.rewarded_set.value;
        if let Some(bond) = rewarded_set
            .iter()
            .find(|mix| mix.mix_node.identity_key == identity)
        {
            return (Some(bond.clone()), MixnodeStatus::Standby);
        }

        let all_bonded = &self.inner.read().await.mixnodes.value;
        if let Some(bond) = all_bonded
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
        self.initialised.load(Ordering::Relaxed)
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
            mixnodes: Cache::default(),
            gateways: Cache::default(),
            rewarded_set: Cache::default(),
            active_set: Cache::default(),
            current_reward_params: Cache::new(EpochRewardParams::new_empty()),
            // setting it to a dummy value on creation is fine, as nothing will be able to ready from it
            // since 'initialised' flag won't be set
            current_epoch: Cache::new(Interval::new(
                u32::MAX,
                OffsetDateTime::UNIX_EPOCH,
                Duration::default(),
            )),
        }
    }
}
