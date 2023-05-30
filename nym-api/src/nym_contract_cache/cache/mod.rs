use crate::support::caching::Cache;
use data::ValidatorCacheData;
use nym_api_requests::models::MixnodeStatus;
use nym_mixnet_contract_common::{
    families::FamilyHead, GatewayBond, IdentityKey, Interval, MixId, MixNodeBond, MixNodeDetails,
    RewardingParams,
};
use nym_name_service_common::NameEntry;
use nym_service_provider_directory_common::ServiceInfo;
use rocket::fairing::AdHoc;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::RwLock;
use tokio::time;

mod data;
pub(crate) mod refresher;

#[derive(Clone)]
pub struct NymContractCache {
    pub(crate) initialised: Arc<AtomicBool>,
    pub(crate) inner: Arc<RwLock<ValidatorCacheData>>,
}

impl NymContractCache {
    fn new() -> Self {
        NymContractCache {
            initialised: Arc::new(AtomicBool::new(false)),
            inner: Arc::new(RwLock::new(ValidatorCacheData::new())),
        }
    }

    pub fn stage() -> AdHoc {
        AdHoc::on_ignite("Validator Cache Stage", |rocket| async {
            rocket.manage(Self::new())
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn update(
        &self,
        mixnodes: Vec<MixNodeDetails>,
        gateways: Vec<GatewayBond>,
        rewarded_set: Vec<MixNodeDetails>,
        active_set: Vec<MixNodeDetails>,
        rewarding_params: RewardingParams,
        current_interval: Interval,
        mix_to_family: Vec<(IdentityKey, FamilyHead)>,
        services: Option<Vec<ServiceInfo>>,
        names: Option<Vec<NameEntry>>,
    ) {
        match time::timeout(Duration::from_millis(100), self.inner.write()).await {
            Ok(mut cache) => {
                cache.mixnodes.update(mixnodes);
                cache.gateways.update(gateways);
                cache.rewarded_set.update(rewarded_set);
                cache.active_set.update(active_set);
                cache.current_reward_params.update(Some(rewarding_params));
                cache.current_interval.update(Some(current_interval));
                cache.mix_to_family.update(mix_to_family);
                // Just return empty lists when these are not available
                cache.service_providers.update(services.unwrap_or_default());
                cache.registered_names.update(names.unwrap_or_default());
            }
            Err(err) => {
                error!("{err}");
            }
        }
    }

    pub async fn mixnodes_blacklist(&self) -> Cache<HashSet<MixId>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.mixnodes_blacklist.clone(),
            Err(err) => {
                error!("{err}");
                Cache::new(HashSet::new())
            }
        }
    }

    pub async fn gateways_blacklist(&self) -> Cache<HashSet<IdentityKey>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.gateways_blacklist.clone(),
            Err(err) => {
                error!("{err}");
                Cache::new(HashSet::new())
            }
        }
    }

    pub async fn update_mixnodes_blacklist(&self, add: HashSet<MixId>, remove: HashSet<MixId>) {
        let blacklist = self.mixnodes_blacklist().await;
        let mut blacklist = blacklist
            .value
            .union(&add)
            .cloned()
            .collect::<HashSet<MixId>>();
        let to_remove = blacklist
            .intersection(&remove)
            .cloned()
            .collect::<HashSet<MixId>>();
        for key in to_remove {
            blacklist.remove(&key);
        }
        match time::timeout(Duration::from_millis(100), self.inner.write()).await {
            Ok(mut cache) => {
                cache.mixnodes_blacklist.update(blacklist);
            }
            Err(err) => {
                error!("Failed to update mixnodes blacklist: {err}");
            }
        }
    }

    pub async fn update_gateways_blacklist(
        &self,
        add: HashSet<IdentityKey>,
        remove: HashSet<IdentityKey>,
    ) {
        let blacklist = self.gateways_blacklist().await;
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
            }
            Err(err) => {
                error!("Failed to update gateways blacklist: {err}");
            }
        }
    }

    pub async fn mixnodes_filtered(&self) -> Vec<MixNodeDetails> {
        let mixnodes = self.mixnodes_all().await;
        if mixnodes.is_empty() {
            return Vec::new();
        }
        let blacklist = self.mixnodes_blacklist().await;

        if !blacklist.is_empty() {
            mixnodes
                .into_iter()
                .filter(|mix| !blacklist.value.contains(&mix.mix_id()))
                .collect()
        } else {
            mixnodes
        }
    }

    pub async fn mixnodes_all(&self) -> Vec<MixNodeDetails> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.mixnodes.clone().value,
            Err(err) => {
                error!("{err}");
                Vec::new()
            }
        }
    }

    pub async fn mixnodes_basic(&self) -> Vec<MixNodeBond> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache
                .mixnodes
                .clone()
                .into_inner()
                .into_iter()
                .map(|bond| bond.bond_information)
                .collect(),
            Err(err) => {
                error!("{err}");
                Vec::new()
            }
        }
    }

    pub async fn gateways_filtered(&self) -> Vec<GatewayBond> {
        let gateways = self.gateways_all().await;
        if gateways.is_empty() {
            return Vec::new();
        }

        let blacklist = self.gateways_blacklist().await;

        if !blacklist.is_empty() {
            gateways
                .into_iter()
                .filter(|mix| !blacklist.value.contains(mix.identity()))
                .collect()
        } else {
            gateways
        }
    }

    pub async fn gateways_all(&self) -> Vec<GatewayBond> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.gateways.value.clone(),
            Err(err) => {
                error!("{err}");
                Vec::new()
            }
        }
    }

    pub async fn rewarded_set(&self) -> Cache<Vec<MixNodeDetails>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.rewarded_set.clone(),
            Err(err) => {
                error!("{err}");
                Cache::new(Vec::new())
            }
        }
    }

    pub async fn active_set(&self) -> Cache<Vec<MixNodeDetails>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.active_set.clone(),
            Err(err) => {
                error!("{err}");
                Cache::new(Vec::new())
            }
        }
    }

    pub async fn mix_to_family(&self) -> Cache<Vec<(IdentityKey, FamilyHead)>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.mix_to_family.clone(),
            Err(err) => {
                error!("{err}");
                Cache::new(Vec::new())
            }
        }
    }

    pub(crate) async fn interval_reward_params(&self) -> Cache<Option<RewardingParams>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.current_reward_params.clone(),
            Err(err) => {
                error!("{err}");
                Cache::new(None)
            }
        }
    }

    pub(crate) async fn current_interval(&self) -> Cache<Option<Interval>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.current_interval.clone(),
            Err(err) => {
                error!("{err}");
                Cache::new(None)
            }
        }
    }

    pub async fn mixnode_details(&self, mix_id: MixId) -> (Option<MixNodeDetails>, MixnodeStatus) {
        // it might not be the most optimal to possibly iterate the entire vector to find (or not)
        // the relevant value. However, the vectors are relatively small (< 10_000 elements, < 1000 for active set)

        let active_set = &self.active_set().await.value;
        if let Some(bond) = active_set.iter().find(|mix| mix.mix_id() == mix_id) {
            return (Some(bond.clone()), MixnodeStatus::Active);
        }

        let rewarded_set = &self.rewarded_set().await.value;
        if let Some(bond) = rewarded_set.iter().find(|mix| mix.mix_id() == mix_id) {
            return (Some(bond.clone()), MixnodeStatus::Standby);
        }

        let all_bonded = &self.mixnodes_filtered().await;
        if let Some(bond) = all_bonded.iter().find(|mix| mix.mix_id() == mix_id) {
            (Some(bond.clone()), MixnodeStatus::Inactive)
        } else {
            (None, MixnodeStatus::NotFound)
        }
    }

    pub async fn mixnode_status(&self, mix_id: MixId) -> MixnodeStatus {
        self.mixnode_details(mix_id).await.1
    }

    pub(crate) async fn services(&self) -> Cache<Vec<ServiceInfo>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.service_providers.clone(),
            Err(err) => {
                error!("{err}");
                Cache::new(Vec::new())
            }
        }
    }

    pub(crate) async fn names(&self) -> Cache<Vec<NameEntry>> {
        match time::timeout(Duration::from_millis(100), self.inner.read()).await {
            Ok(cache) => cache.registered_names.clone(),
            Err(err) => {
                error!("{err}");
                Cache::new(Vec::new())
            }
        }
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
