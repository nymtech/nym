// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::RefreshData;
use crate::nym_contract_cache::cache::data::CachedContractsInfo;
use crate::support::caching::Cache;
use data::ValidatorCacheData;
use nym_api_requests::legacy::{
    LegacyGatewayBondWithId, LegacyMixNodeBondWithLayer, LegacyMixNodeDetailsWithLayer,
};
use nym_api_requests::models::MixnodeStatus;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::{Interval, NodeId, NymNodeDetails, RewardedSet, RewardingParams};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::time;
use tracing::{debug, error};

mod data;
pub(crate) mod refresher;

pub(crate) use self::data::CachedRewardedSet;

const CACHE_TIMEOUT_MS: u64 = 100;

pub(crate) struct RefreshDataWithKey {
    pub(crate) pubkey: ed25519::PublicKey,
    pub(crate) refresh_data: RefreshData,
}

#[derive(Clone)]
pub struct NymContractCache {
    pub(crate) initialised: Arc<AtomicBool>,
    pub(crate) inner: Arc<RwLock<ValidatorCacheData>>,
}

impl NymContractCache {
    pub(crate) fn new() -> Self {
        NymContractCache {
            initialised: Arc::new(AtomicBool::new(false)),
            inner: Arc::new(RwLock::new(ValidatorCacheData::new())),
        }
    }

    /// Returns a copy of the current cache data.
    async fn get_owned<T>(
        &self,
        fn_arg: impl FnOnce(RwLockReadGuard<'_, ValidatorCacheData>) -> Cache<T>,
    ) -> Option<Cache<T>> {
        match time::timeout(Duration::from_millis(CACHE_TIMEOUT_MS), self.inner.read()).await {
            Ok(cache) => Some(fn_arg(cache)),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }

    async fn get<'a, T: 'a>(
        &'a self,
        fn_arg: impl FnOnce(&ValidatorCacheData) -> &Cache<T>,
    ) -> Option<RwLockReadGuard<'a, Cache<T>>> {
        match time::timeout(Duration::from_millis(CACHE_TIMEOUT_MS), self.inner.read()).await {
            Ok(cache) => Some(RwLockReadGuard::map(cache, |item| fn_arg(item))),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn update(
        &self,
        mixnodes: Vec<LegacyMixNodeDetailsWithLayer>,
        gateways: Vec<LegacyGatewayBondWithId>,
        nym_nodes: Vec<NymNodeDetails>,
        rewarded_set: RewardedSet,
        rewarding_params: RewardingParams,
        current_interval: Interval,
        nym_contracts_info: CachedContractsInfo,
    ) {
        match time::timeout(Duration::from_millis(100), self.inner.write()).await {
            Ok(mut cache) => {
                cache.legacy_mixnodes.unchecked_update(mixnodes);
                cache.legacy_gateways.unchecked_update(gateways);
                cache.nym_nodes.unchecked_update(nym_nodes);
                cache.rewarded_set.unchecked_update(rewarded_set);
                cache
                    .current_reward_params
                    .unchecked_update(Some(rewarding_params));
                cache
                    .current_interval
                    .unchecked_update(Some(current_interval));
                cache.contracts_info.unchecked_update(nym_contracts_info)
            }
            Err(err) => {
                error!("{err}");
            }
        }
    }

    pub async fn mixnodes_blacklist(&self) -> Cache<HashSet<NodeId>> {
        self.get_owned(|cache| cache.legacy_mixnodes_blacklist.clone_cache())
            .await
            .unwrap_or_default()
    }

    pub async fn gateways_blacklist(&self) -> Cache<HashSet<NodeId>> {
        self.get_owned(|cache| cache.legacy_gateways_blacklist.clone_cache())
            .await
            .unwrap_or_default()
    }

    pub async fn update_mixnodes_blacklist(&self, add: HashSet<NodeId>, remove: HashSet<NodeId>) {
        let blacklist = self.mixnodes_blacklist().await;
        let mut blacklist = blacklist.union(&add).cloned().collect::<HashSet<NodeId>>();
        let to_remove = blacklist
            .intersection(&remove)
            .cloned()
            .collect::<HashSet<NodeId>>();
        for key in to_remove {
            blacklist.remove(&key);
        }
        match time::timeout(Duration::from_millis(100), self.inner.write()).await {
            Ok(mut cache) => {
                cache.legacy_mixnodes_blacklist.unchecked_update(blacklist);
            }
            Err(err) => {
                error!("Failed to update mixnodes blacklist: {err}");
            }
        }
    }

    pub async fn update_gateways_blacklist(&self, add: HashSet<NodeId>, remove: HashSet<NodeId>) {
        let blacklist = self.gateways_blacklist().await;
        let mut blacklist = blacklist.union(&add).cloned().collect::<HashSet<_>>();
        let to_remove = blacklist
            .intersection(&remove)
            .cloned()
            .collect::<HashSet<_>>();
        for key in to_remove {
            blacklist.remove(&key);
        }
        match time::timeout(Duration::from_millis(100), self.inner.write()).await {
            Ok(mut cache) => {
                cache.legacy_gateways_blacklist.unchecked_update(blacklist);
            }
            Err(err) => {
                error!("Failed to update gateways blacklist: {err}");
            }
        }
    }

    pub async fn legacy_mixnodes_filtered(&self) -> Vec<LegacyMixNodeDetailsWithLayer> {
        let mixnodes = self.legacy_mixnodes_all().await;
        if mixnodes.is_empty() {
            return Vec::new();
        }
        let blacklist = self.mixnodes_blacklist().await;

        if !blacklist.is_empty() {
            mixnodes
                .into_iter()
                .filter(|mix| !blacklist.contains(&mix.mix_id()))
                .collect()
        } else {
            mixnodes
        }
    }

    pub async fn all_cached_legacy_mixnodes(
        &self,
    ) -> Option<RwLockReadGuard<Cache<Vec<LegacyMixNodeDetailsWithLayer>>>> {
        self.get(|c| &c.legacy_mixnodes).await
    }

    pub async fn legacy_gateway_owner(&self, node_id: NodeId) -> Option<String> {
        self.get(|c| &c.legacy_gateways)
            .await?
            .iter()
            .find(|g| g.node_id == node_id)
            .map(|g| g.owner.to_string())
    }

    #[allow(dead_code)]
    pub async fn legacy_mixnode_owner(&self, node_id: NodeId) -> Option<String> {
        self.get(|c| &c.legacy_mixnodes)
            .await?
            .iter()
            .find(|m| m.mix_id() == node_id)
            .map(|m| m.bond_information.owner.to_string())
    }

    pub async fn all_cached_legacy_gateways(
        &self,
    ) -> Option<RwLockReadGuard<Cache<Vec<LegacyGatewayBondWithId>>>> {
        self.get(|c| &c.legacy_gateways).await
    }

    pub async fn all_cached_nym_nodes(
        &self,
    ) -> Option<RwLockReadGuard<Cache<Vec<NymNodeDetails>>>> {
        self.get(|c| &c.nym_nodes).await
    }

    pub async fn legacy_mixnodes_all(&self) -> Vec<LegacyMixNodeDetailsWithLayer> {
        self.get_owned(|cache| cache.legacy_mixnodes.clone_cache())
            .await
            .unwrap_or_default()
            .into_inner()
    }

    pub async fn legacy_mixnodes_filtered_basic(&self) -> Vec<LegacyMixNodeBondWithLayer> {
        self.legacy_mixnodes_filtered()
            .await
            .into_iter()
            .map(|bond| bond.bond_information)
            .collect()
    }

    pub async fn legacy_mixnodes_all_basic(&self) -> Vec<LegacyMixNodeBondWithLayer> {
        self.legacy_mixnodes_all()
            .await
            .into_iter()
            .map(|bond| bond.bond_information)
            .collect()
    }

    pub async fn legacy_gateways_filtered(&self) -> Vec<LegacyGatewayBondWithId> {
        let gateways = self.legacy_gateways_all().await;
        if gateways.is_empty() {
            return Vec::new();
        }

        let blacklist = self.gateways_blacklist().await;

        if !blacklist.is_empty() {
            gateways
                .into_iter()
                .filter(|gw| !blacklist.contains(&gw.node_id))
                .collect()
        } else {
            gateways
        }
    }

    pub async fn legacy_gateways_all(&self) -> Vec<LegacyGatewayBondWithId> {
        self.get_owned(|cache| cache.legacy_gateways.clone_cache())
            .await
            .unwrap_or_default()
            .into_inner()
    }

    pub async fn nym_nodes(&self) -> Vec<NymNodeDetails> {
        self.get_owned(|cache| cache.nym_nodes.clone_cache())
            .await
            .unwrap_or_default()
            .into_inner()
    }

    pub async fn rewarded_set(&self) -> Option<RwLockReadGuard<Cache<CachedRewardedSet>>> {
        self.get(|cache| &cache.rewarded_set).await
    }

    pub async fn rewarded_set_owned(&self) -> Cache<CachedRewardedSet> {
        self.get_owned(|cache| cache.rewarded_set.clone_cache())
            .await
            .unwrap_or_default()
    }

    pub async fn legacy_v1_rewarded_set_mixnodes(&self) -> Vec<LegacyMixNodeDetailsWithLayer> {
        let Some(rewarded_set) = self.rewarded_set().await else {
            return Vec::new();
        };

        let mut rewarded_nodes = rewarded_set
            .active_mixnodes()
            .into_iter()
            .collect::<HashSet<_>>();

        // rewarded mixnode = active or standby
        for standby in &rewarded_set.standby {
            rewarded_nodes.insert(*standby);
        }

        self.legacy_mixnodes_all()
            .await
            .into_iter()
            .filter(|m| rewarded_nodes.contains(&m.mix_id()))
            .collect()
    }

    pub async fn legacy_v1_active_set_mixnodes(&self) -> Vec<LegacyMixNodeDetailsWithLayer> {
        let Some(rewarded_set) = self.rewarded_set().await else {
            return Vec::new();
        };

        let active_nodes = rewarded_set
            .active_mixnodes()
            .into_iter()
            .collect::<HashSet<_>>();

        self.legacy_mixnodes_all()
            .await
            .into_iter()
            .filter(|m| active_nodes.contains(&m.mix_id()))
            .collect()
    }

    pub(crate) async fn interval_reward_params(&self) -> Cache<Option<RewardingParams>> {
        self.get_owned(|cache| cache.current_reward_params.clone_cache())
            .await
            .unwrap_or_default()
    }

    pub(crate) async fn current_interval(&self) -> Cache<Option<Interval>> {
        self.get_owned(|cache| cache.current_interval.clone_cache())
            .await
            .unwrap_or_default()
    }

    pub(crate) async fn contract_details(&self) -> Cache<CachedContractsInfo> {
        self.get_owned(|cache| cache.contracts_info.clone_cache())
            .await
            .unwrap_or_default()
    }

    pub async fn legacy_mixnode_details(
        &self,
        mix_id: NodeId,
    ) -> (Option<LegacyMixNodeDetailsWithLayer>, MixnodeStatus) {
        // the old behaviour was to get the nodes from the filtered list, so let's not change it here
        let rewarded_set = self.rewarded_set_owned().await;
        let all_bonded = &self.legacy_mixnodes_filtered().await;
        let Some(bond) = all_bonded.iter().find(|mix| mix.mix_id() == mix_id) else {
            return (None, MixnodeStatus::NotFound);
        };

        if rewarded_set.is_active_mixnode(&mix_id) {
            return (Some(bond.clone()), MixnodeStatus::Active);
        }

        if rewarded_set.is_standby(&mix_id) {
            return (Some(bond.clone()), MixnodeStatus::Standby);
        }

        (Some(bond.clone()), MixnodeStatus::Inactive)
    }

    pub async fn mixnode_status(&self, mix_id: NodeId) -> MixnodeStatus {
        self.legacy_mixnode_details(mix_id).await.1
    }

    pub async fn get_public_key_with_refresh_data(
        &self,
        node_id: NodeId,
    ) -> Option<RefreshDataWithKey> {
        if !self.initialised() {
            return None;
        }

        let inner = self.inner.read().await;

        // 1. check nymnodes
        if let Some(nym_node) = inner.nym_nodes.iter().find(|n| n.node_id() == node_id) {
            let pubkey = nym_node.bond_information.identity().parse().ok()?;
            return Some(RefreshDataWithKey {
                pubkey,
                refresh_data: nym_node.into(),
            });
        }

        // 2. check legacy mixnodes
        if let Some(mixnode) = inner.legacy_mixnodes.iter().find(|n| n.mix_id() == node_id) {
            let pubkey = mixnode.bond_information.identity().parse().ok()?;
            return Some(RefreshDataWithKey {
                pubkey,
                refresh_data: mixnode.into(),
            });
        }

        // 3. check legacy gateways
        if let Some(gateway) = inner.legacy_gateways.iter().find(|n| n.node_id == node_id) {
            let pubkey = gateway.identity().parse().ok()?;
            return Some(RefreshDataWithKey {
                pubkey,
                refresh_data: gateway.into(),
            });
        }

        None
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
