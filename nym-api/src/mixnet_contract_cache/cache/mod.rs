// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::data::ConfigScoreData;
use crate::node_describe_cache::refresh::RefreshData;
use crate::support::caching::cache::{SharedCache, UninitialisedCache};
use crate::support::caching::Cache;
use data::MixnetContractCacheData;
use nym_api_requests::legacy::{
    LegacyGatewayBondWithId, LegacyMixNodeBondWithLayer, LegacyMixNodeDetailsWithLayer,
};
use nym_api_requests::models::{CirculatingSupplyResponse, MixnodeStatus};
use nym_contracts_common::truncate_decimal;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::{
    Interval, KeyRotationState, NodeId, NymNodeDetails, RewardingParams,
};
use nym_topology::CachedEpochRewardedSet;
use nym_validator_client::nyxd::Coin;
use time::OffsetDateTime;
use tokio::sync::RwLockReadGuard;

pub(crate) mod data;
pub(crate) mod refresher;

const TOTAL_SUPPLY_AMOUNT: u128 = 1_000_000_000_000_000; // 1B tokens

#[derive(Clone)]
pub struct MixnetContractCache {
    pub(crate) inner: SharedCache<MixnetContractCacheData>,
}

impl MixnetContractCache {
    pub(crate) fn new() -> Self {
        MixnetContractCache {
            inner: SharedCache::new(),
        }
    }

    pub(crate) fn inner(&self) -> SharedCache<MixnetContractCacheData> {
        self.inner.clone()
    }

    async fn get_owned<T>(
        &self,
        fn_arg: impl FnOnce(&MixnetContractCacheData) -> T,
    ) -> Result<T, UninitialisedCache> {
        Ok(fn_arg(&**self.inner.get().await?))
    }

    async fn get<'a, T: 'a>(
        &'a self,
        fn_arg: impl FnOnce(&Cache<MixnetContractCacheData>) -> &T,
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
    ) -> Option<RwLockReadGuard<'_, Vec<LegacyMixNodeDetailsWithLayer>>> {
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
    ) -> Option<RwLockReadGuard<'_, Vec<LegacyGatewayBondWithId>>> {
        self.get(|c| &c.legacy_gateways).await.ok()
    }

    pub async fn all_cached_nym_nodes(&self) -> Option<RwLockReadGuard<'_, Vec<NymNodeDetails>>> {
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

    pub async fn rewarded_set(&self) -> Option<RwLockReadGuard<'_, CachedEpochRewardedSet>> {
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

    pub(crate) async fn current_key_rotation_id(&self) -> Result<u32, UninitialisedCache> {
        let guard = self.inner.get().await?;
        let current_absolute_epoch_id = guard.current_interval.current_epoch_absolute_id();
        Ok(guard
            .key_rotation_state
            .key_rotation_id(current_absolute_epoch_id))
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

    pub(crate) async fn get_circulating_supply(&self) -> Option<CirculatingSupplyResponse> {
        let mix_denom = self.get_owned(|c| c.rewarding_denom.clone()).await.ok()?;
        let reward_pool = self
            .interval_reward_params()
            .await
            .ok()?
            .interval
            .reward_pool;

        let mixmining_reserve_amount = truncate_decimal(reward_pool).u128();
        let mixmining_reserve = Coin::new(mixmining_reserve_amount, &mix_denom).into();

        // given all tokens have already vested, the circulating supply is total supply - mixmining reserve
        let circulating_supply =
            Coin::new(TOTAL_SUPPLY_AMOUNT - mixmining_reserve_amount, &mix_denom).into();

        Some(CirculatingSupplyResponse {
            total_supply: Coin::new(TOTAL_SUPPLY_AMOUNT, &mix_denom).into(),
            mixmining_reserve,
            // everything has already vested
            vesting_tokens: Coin::new(0, &mix_denom).into(),
            circulating_supply,
        })
    }

    pub(crate) async fn naive_wait_for_initial_values(&self) {
        self.inner.naive_wait_for_initial_values().await
    }
}
