// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::RefreshData;
use crate::nym_contract_cache::cache::data::{CachedContractsInfo, ConfigScoreData};
use crate::support::caching::cache::{SharedCache, UninitialisedCache};
use crate::support::caching::Cache;
use data::ContractCacheData;
use nym_api_requests::legacy::{
    LegacyGatewayBondWithId, LegacyMixNodeBondWithLayer, LegacyMixNodeDetailsWithLayer,
};
use nym_api_requests::models::MixnodeStatus;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::{
    Interval, KeyRotationState, NodeId, NymNodeDetails, RewardingParams,
};
use nym_topology::CachedEpochRewardedSet;
use time::OffsetDateTime;
use tokio::sync::RwLockReadGuard;

pub(crate) mod data;
pub(crate) mod refresher;

#[derive(Clone)]
pub struct NymContractCache {
    pub(crate) inner: SharedCache<ContractCacheData>,
}

impl NymContractCache {
    pub(crate) fn new() -> Self {
        NymContractCache {
            inner: SharedCache::new(),
        }
    }

    pub(crate) fn inner(&self) -> SharedCache<ContractCacheData> {
        self.inner.clone()
    }

    async fn get_owned<T>(
        &self,
        fn_arg: impl FnOnce(&ContractCacheData) -> T,
    ) -> Result<T, UninitialisedCache> {
        Ok(fn_arg(&**self.inner.get().await?))
    }

    async fn get<'a, T: 'a>(
        &'a self,
        fn_arg: impl FnOnce(&Cache<ContractCacheData>) -> &T,
    ) -> Result<RwLockReadGuard<'a, T>, UninitialisedCache> {
        let guard = self.inner.get().await?;
        Ok(RwLockReadGuard::map(guard, fn_arg))
    }

    pub async fn cache_timestamp(&self) -> OffsetDateTime {
        let Ok(cache) = self.inner.get().await else {
            return OffsetDateTime::UNIX_EPOCH;
        };

        cache.timestamp()
    }

    pub async fn all_cached_legacy_mixnodes(
        &self,
    ) -> Option<RwLockReadGuard<Vec<LegacyMixNodeDetailsWithLayer>>> {
        self.get(|c| &c.legacy_mixnodes).await.ok()
    }

    pub async fn legacy_gateway_owner(&self, node_id: NodeId) -> Option<String> {
        let Ok(cache) = self.inner.get().await else {
            return Default::default();
        };

        cache
            .legacy_gateways
            .iter()
            .find(|gateway| gateway.node_id == node_id)
            .map(|gateway| gateway.owner.to_string())
    }

    pub async fn all_cached_legacy_gateways(
        &self,
    ) -> Option<RwLockReadGuard<Vec<LegacyGatewayBondWithId>>> {
        self.get(|c| &c.legacy_gateways).await.ok()
    }

    pub async fn all_cached_nym_nodes(&self) -> Option<RwLockReadGuard<Vec<NymNodeDetails>>> {
        self.get(|c| &c.nym_nodes).await.ok()
    }

    pub async fn legacy_mixnodes_all(&self) -> Vec<LegacyMixNodeDetailsWithLayer> {
        self.get_owned(|c| c.legacy_mixnodes.clone())
            .await
            .unwrap_or_default()
    }

    pub async fn legacy_mixnodes_all_basic(&self) -> Vec<LegacyMixNodeBondWithLayer> {
        self.legacy_mixnodes_all()
            .await
            .into_iter()
            .map(|bond| bond.bond_information)
            .collect()
    }

    pub async fn legacy_gateways_all(&self) -> Vec<LegacyGatewayBondWithId> {
        self.get_owned(|c| c.legacy_gateways.clone())
            .await
            .unwrap_or_default()
    }

    pub async fn nym_nodes(&self) -> Vec<NymNodeDetails> {
        self.get_owned(|c| c.nym_nodes.clone())
            .await
            .unwrap_or_default()
    }

    pub async fn cached_rewarded_set(
        &self,
    ) -> Result<Cache<CachedEpochRewardedSet>, UninitialisedCache> {
        let cache = self.inner.get().await?;
        Ok(Cache::as_mapped(&cache, |c| c.rewarded_set.clone()))
    }

    pub async fn rewarded_set(&self) -> Option<RwLockReadGuard<CachedEpochRewardedSet>> {
        self.get(|c| &c.rewarded_set).await.ok()
    }

    pub async fn rewarded_set_owned(&self) -> Result<CachedEpochRewardedSet, UninitialisedCache> {
        self.get_owned(|c| c.rewarded_set.clone()).await
    }

    pub async fn maybe_config_score_data(&self) -> Result<ConfigScoreData, UninitialisedCache> {
        self.get_owned(|c| c.config_score_data.clone()).await
    }

    pub(crate) async fn interval_reward_params(
        &self,
    ) -> Result<RewardingParams, UninitialisedCache> {
        self.get_owned(|c| c.current_reward_params).await
    }

    pub(crate) async fn current_interval(&self) -> Result<Interval, UninitialisedCache> {
        self.get_owned(|c| c.current_interval).await
    }

    pub(crate) async fn get_key_rotation_state(
        &self,
    ) -> Result<KeyRotationState, UninitialisedCache> {
        self.get_owned(|c| c.key_rotation_state).await
    }

    pub(crate) async fn contract_details(&self) -> CachedContractsInfo {
        self.get_owned(|c| c.contracts_info.clone())
            .await
            .unwrap_or_default()
    }

    pub async fn mixnode_status(&self, mix_id: NodeId) -> MixnodeStatus {
        let Ok(cache) = self.inner.get().await else {
            return Default::default();
        };

        if cache.legacy_mixnodes.iter().any(|n| n.mix_id() == mix_id) {
            MixnodeStatus::Inactive
        } else {
            MixnodeStatus::NotFound
        }
    }

    pub async fn get_node_refresh_data(
        &self,
        node_identity: ed25519::PublicKey,
    ) -> Option<RefreshData> {
        let Ok(cache) = self.inner.get().await else {
            return Default::default();
        };

        let encoded_identity = node_identity.to_base58_string();

        // 1. check nymnodes
        if let Some(nym_node) = cache
            .nym_nodes
            .iter()
            .find(|n| n.bond_information.identity() == encoded_identity)
        {
            return nym_node.try_into().ok();
        }

        // 2. check legacy mixnodes
        if let Some(mixnode) = cache
            .legacy_mixnodes
            .iter()
            .find(|n| n.bond_information.identity() == encoded_identity)
        {
            return mixnode.try_into().ok();
        }

        // 3. check legacy gateways
        if let Some(gateway) = cache
            .legacy_gateways
            .iter()
            .find(|n| n.identity() == &encoded_identity)
        {
            return gateway.try_into().ok();
        }

        None
    }

    pub(crate) async fn naive_wait_for_initial_values(&self) {
        self.inner.naive_wait_for_initial_values().await
    }
}
