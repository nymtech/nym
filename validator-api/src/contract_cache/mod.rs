// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::Uptime;
use crate::nymd_client::Client;
use crate::storage::ValidatorApiStorage;
use ::time::OffsetDateTime;
use anyhow::Result;
use mixnet_contract_common::reward_params::EpochRewardParams;
use mixnet_contract_common::{
    GatewayBond, IdentityKey, IdentityKeyRef, Interval, MixNodeBond, RewardedSetNodeStatus,
};
use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;
use task::ShutdownListener;

use rocket::fairing::AdHoc;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use validator_api_requests::models::{MixNodeBondAnnotated, MixnodeStatus};
use validator_client::nymd::CosmWasmClient;

pub(crate) mod reward_estimate;
pub(crate) mod routes;

pub struct ValidatorCacheRefresher<C> {
    nymd_client: Client<C>,
    cache: ValidatorCache,
    caching_interval: Duration,

    // Readonly: some of the quantities cached depends on values from the storage.
    storage: Option<ValidatorApiStorage>,
}

#[derive(Clone)]
pub struct ValidatorCache {
    initialised: Arc<AtomicBool>,
    inner: Arc<RwLock<ValidatorCacheInner>>,
}

struct ValidatorCacheInner {
    mixnodes: Cache<Vec<MixNodeBondAnnotated>>,
    gateways: Cache<Vec<GatewayBond>>,

    mixnodes_blacklist: Cache<HashSet<IdentityKey>>,
    gateways_blacklist: Cache<HashSet<IdentityKey>>,

    rewarded_set: Cache<Vec<MixNodeBondAnnotated>>,
    active_set: Cache<Vec<MixNodeBondAnnotated>>,

    current_reward_params: Cache<EpochRewardParams>,
    current_epoch: Cache<Option<Interval>>,
    current_operator_base_cost: Cache<u64>,
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
        storage: Option<ValidatorApiStorage>,
    ) -> Self {
        ValidatorCacheRefresher {
            nymd_client,
            cache,
            caching_interval,
            storage,
        }
    }

    async fn get_uptime(&self, identity: &IdentityKey, epoch: Interval) -> Option<Uptime> {
        self.storage
            .as_ref()?
            .get_average_mixnode_uptime_in_the_last_24hrs(identity, epoch.end_unix_timestamp())
            .await
            .ok()
    }

    async fn annotate_bond_with_details(
        &self,
        mixnodes: Vec<MixNodeBond>,
        interval_reward_params: EpochRewardParams,
        current_epoch: Interval,
        epochs_in_interval: u64,
        current_operator_base_cost: u64,
        rewarded_set_identities: &HashMap<IdentityKey, RewardedSetNodeStatus>,
    ) -> Vec<MixNodeBondAnnotated> {
        let mut annotated = Vec::new();
        for mixnode_bond in mixnodes {
            let stake_saturation = mixnode_bond
                .stake_saturation(
                    interval_reward_params.staking_supply(),
                    interval_reward_params.rewarded_set_size() as u32,
                )
                .to_num();

            let uptime = self
                .get_uptime(mixnode_bond.identity(), current_epoch)
                .await
                .unwrap_or_default();

            let is_active = rewarded_set_identities
                .get(mixnode_bond.identity())
                .map_or(false, RewardedSetNodeStatus::is_active);

            let reward_estimate = reward_estimate::compute_reward_estimate(
                &mixnode_bond,
                uptime,
                is_active,
                interval_reward_params,
                current_operator_base_cost,
            );

            let (estimated_operator_apy, estimated_delegators_apy) =
                reward_estimate::compute_apy_from_reward(
                    &mixnode_bond,
                    reward_estimate,
                    epochs_in_interval,
                );

            annotated.push(MixNodeBondAnnotated {
                mixnode_bond,
                stake_saturation,
                uptime: uptime.u8(),
                estimated_operator_apy,
                estimated_delegators_apy,
            });
        }
        annotated
    }

    async fn get_rewarded_set_identities(&self) -> HashMap<String, RewardedSetNodeStatus>
    where
        C: CosmWasmClient + Sync,
    {
        if let Ok(rewarded_set_identities) = self.nymd_client.get_rewarded_set_identities().await {
            rewarded_set_identities
                .into_iter()
                .collect::<HashMap<_, _>>()
        } else {
            HashMap::new()
        }
    }

    fn collect_rewarded_and_active_set_details(
        all_mixnodes: &[MixNodeBondAnnotated],
        rewarded_set_identities: &HashMap<IdentityKey, RewardedSetNodeStatus>,
    ) -> (Vec<MixNodeBondAnnotated>, Vec<MixNodeBondAnnotated>) {
        let mut active_set = Vec::new();
        let mut rewarded_set = Vec::new();

        for mix in all_mixnodes {
            if let Some(status) = rewarded_set_identities.get(mix.mixnode_bond.identity()) {
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
        let epoch_rewarding_params = self.nymd_client.get_current_epoch_reward_params().await?;
        let current_epoch = self.nymd_client.get_current_epoch().await?;
        let current_operator_base_cost = self.nymd_client.get_current_operator_cost().await?;
        let epochs_in_interval = self.nymd_client.get_epochs_in_interval().await.unwrap_or(0);

        let (mixnodes, gateways) = tokio::try_join!(
            self.nymd_client.get_mixnodes(),
            self.nymd_client.get_gateways(),
        )?;

        let rewarded_set_identities = self.get_rewarded_set_identities().await;

        let mixnodes = self
            .annotate_bond_with_details(
                mixnodes,
                epoch_rewarding_params,
                current_epoch,
                epochs_in_interval,
                current_operator_base_cost,
                &rewarded_set_identities,
            )
            .await;

        let (rewarded_set, active_set) =
            Self::collect_rewarded_and_active_set_details(&mixnodes, &rewarded_set_identities);

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
                current_epoch,
                current_operator_base_cost,
            )
            .await;

        Ok(())
    }

    pub(crate) async fn run(&self, mut shutdown: ShutdownListener)
    where
        C: CosmWasmClient + Sync,
    {
        let mut interval = time::interval(self.caching_interval);
        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(err) = self.refresh_cache().await {
                        error!("Failed to refresh validator cache - {}", err);
                    } else {
                        // relaxed memory ordering is fine here. worst case scenario network monitor
                        // will just have to wait for an additional backoff to see the change.
                        // And so this will not really incur any performance penalties by setting it every loop iteration
                        self.cache.initialised.store(true, Ordering::Relaxed)
                    }
                }
                _ = shutdown.recv() => {
                    trace!("UpdateHandler: Received shutdown");
                }
            }
        }
    }
}

pub(crate) fn validator_cache_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: routes::get_mixnodes,
        routes::get_mixnodes_detailed,
        routes::get_gateways,
        routes::get_active_set,
        routes::get_active_set_detailed,
        routes::get_rewarded_set,
        routes::get_rewarded_set_detailed,
        routes::get_blacklisted_mixnodes,
        routes::get_blacklisted_gateways,
        routes::get_epoch_reward_params,
        routes::get_current_epoch
    ]
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
            rocket.manage(Self::new())
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn update_cache(
        &self,
        mixnodes: Vec<MixNodeBondAnnotated>,
        gateways: Vec<GatewayBond>,
        rewarded_set: Vec<MixNodeBondAnnotated>,
        active_set: Vec<MixNodeBondAnnotated>,
        epoch_rewarding_params: EpochRewardParams,
        current_epoch: Interval,
        current_operator_base_cost: u64,
    ) {
        match time::timeout(Duration::from_millis(100), self.inner.write()).await {
            Ok(mut cache) => {
                cache.mixnodes.update(mixnodes);
                cache.gateways.update(gateways);
                cache.rewarded_set.update(rewarded_set);
                cache.active_set.update(active_set);
                cache.current_reward_params.update(epoch_rewarding_params);
                cache.current_epoch.update(Some(current_epoch));
                cache
                    .current_operator_base_cost
                    .update(current_operator_base_cost);
            }
            Err(e) => {
                error!("{}", e);
            }
        }
    }

    pub async fn mixnodes_blacklist(&self) -> Option<Cache<HashSet<IdentityKey>>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => Some(cache.mixnodes_blacklist.clone()),
            Err(e) => {
                error!("{}", e);
                None
            }
        }
    }

    pub async fn gateways_blacklist(&self) -> Option<Cache<HashSet<IdentityKey>>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => Some(cache.gateways_blacklist.clone()),
            Err(e) => {
                error!("{}", e);
                None
            }
        }
    }

    pub async fn update_mixnodes_blacklist(
        &self,
        add: HashSet<IdentityKey>,
        remove: HashSet<IdentityKey>,
    ) {
        let blacklist = self.mixnodes_blacklist().await;
        if let Some(blacklist) = blacklist {
            let mut blacklist = blacklist
                .value
                .union(&add)
                .cloned()
                .collect::<HashSet<IdentityKey>>();
            let to_remove = blacklist
                .intersection(&remove)
                .cloned()
                .collect::<HashSet<IdentityKey>>();
            for key in to_remove {
                blacklist.remove(&key);
            }
            match time::timeout(Duration::from_millis(100), self.inner.write()).await {
                Ok(mut cache) => {
                    cache.mixnodes_blacklist.update(blacklist);
                    return;
                }
                Err(e) => error!("{}", e),
            }
        }
        error!("Failed to update mixnodes blacklist");
    }

    pub async fn update_gateways_blacklist(
        &self,
        add: HashSet<IdentityKey>,
        remove: HashSet<IdentityKey>,
    ) {
        let blacklist = self.gateways_blacklist().await;
        if let Some(blacklist) = blacklist {
            let mut blacklist = blacklist
                .value
                .union(&add)
                .cloned()
                .collect::<HashSet<IdentityKey>>();
            let to_remove = blacklist
                .intersection(&remove)
                .cloned()
                .collect::<HashSet<IdentityKey>>();
            for key in to_remove {
                blacklist.remove(&key);
            }
            match time::timeout(Duration::from_millis(100), self.inner.write()).await {
                Ok(mut cache) => {
                    cache.gateways_blacklist.update(blacklist);
                    return;
                }
                Err(e) => error!("{}", e),
            }
        }
        error!("Failed to update gateways blacklist");
    }

    pub async fn mixnodes_detailed(&self) -> Vec<MixNodeBondAnnotated> {
        let blacklist = self.mixnodes_blacklist().await;
        let mixnodes = match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.mixnodes.clone(),
            Err(e) => {
                error!("{}", e);
                return Vec::new();
            }
        };

        if let Some(blacklist) = blacklist {
            mixnodes
                .value
                .iter()
                .filter(|mix| !blacklist.value.contains(mix.mixnode_bond.identity()))
                .cloned()
                .collect()
        } else {
            mixnodes.value
        }
    }

    pub async fn mixnodes(&self) -> Vec<MixNodeBond> {
        self.mixnodes_detailed()
            .await
            .into_iter()
            .map(|bond| bond.mixnode_bond)
            .collect()
    }

    pub async fn mixnodes_all(&self) -> Vec<MixNodeBond> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache
                .mixnodes
                .clone()
                .into_inner()
                .into_iter()
                .map(|bond| bond.mixnode_bond)
                .collect(),
            Err(e) => {
                error!("{}", e);
                Vec::new()
            }
        }
    }

    pub async fn gateways(&self) -> Vec<GatewayBond> {
        let blacklist = self.gateways_blacklist().await;
        let gateways = match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.gateways.clone(),
            Err(e) => {
                error!("{}", e);
                return Vec::new();
            }
        };

        if let Some(blacklist) = blacklist {
            gateways
                .value
                .iter()
                .filter(|mix| !blacklist.value.contains(mix.identity()))
                .cloned()
                .collect()
        } else {
            gateways.value
        }
    }

    pub async fn gateways_all(&self) -> Vec<GatewayBond> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.gateways.value.clone(),
            Err(e) => {
                error!("{}", e);
                Vec::new()
            }
        }
    }

    pub async fn rewarded_set_detailed(&self) -> Cache<Vec<MixNodeBondAnnotated>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.rewarded_set.clone(),
            Err(e) => {
                error!("{}", e);
                Cache::new(Vec::new())
            }
        }
    }

    pub async fn rewarded_set(&self) -> Vec<MixNodeBond> {
        self.rewarded_set_detailed()
            .await
            .value
            .into_iter()
            .map(|bond| bond.mixnode_bond)
            .collect()
    }

    pub async fn active_set_detailed(&self) -> Cache<Vec<MixNodeBondAnnotated>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.active_set.clone(),
            Err(e) => {
                error!("{}", e);
                Cache::new(Vec::new())
            }
        }
    }

    pub async fn active_set(&self) -> Vec<MixNodeBond> {
        self.active_set_detailed()
            .await
            .value
            .into_iter()
            .map(|bond| bond.mixnode_bond)
            .collect()
    }

    pub(crate) async fn epoch_reward_params(&self) -> Cache<EpochRewardParams> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.current_reward_params.clone(),
            Err(e) => {
                error!("{}", e);
                Cache::new(EpochRewardParams::new_empty())
            }
        }
    }

    pub(crate) async fn current_epoch(&self) -> Cache<Option<Interval>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.current_epoch.clone(),
            Err(e) => {
                error!("{}", e);
                Cache::new(None)
            }
        }
    }

    pub(crate) async fn base_operator_cost(&self) -> Cache<u64> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.current_operator_base_cost.clone(),
            Err(e) => {
                error!("{}", e);
                Cache::new(0)
            }
        }
    }

    pub async fn mixnode_details(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> (Option<MixNodeBondAnnotated>, MixnodeStatus) {
        // it might not be the most optimal to possibly iterate the entire vector to find (or not)
        // the relevant value. However, the vectors are relatively small (< 10_000 elements, < 1000 for active set)

        let active_set = &self.active_set_detailed().await.value;
        if let Some(bond) = active_set
            .iter()
            .find(|mix| mix.mix_node().identity_key == identity)
        {
            return (Some(bond.clone()), MixnodeStatus::Active);
        }

        let rewarded_set = &self.rewarded_set_detailed().await.value;
        if let Some(bond) = rewarded_set
            .iter()
            .find(|mix| mix.mix_node().identity_key == identity)
        {
            return (Some(bond.clone()), MixnodeStatus::Standby);
        }

        let all_bonded = &self.mixnodes_detailed().await;
        if let Some(bond) = all_bonded
            .iter()
            .find(|mix| mix.mix_node().identity_key == identity)
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
            mixnodes_blacklist: Cache::default(),
            gateways_blacklist: Cache::default(),
            // setting it to a dummy value on creation is fine, as nothing will be able to ready from it
            // since 'initialised' flag won't be set
            current_epoch: Cache::new(None),
            current_operator_base_cost: Cache::new(0),
        }
    }
}
