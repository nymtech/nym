// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::data::NodeStatusCacheData;
use self::inclusion_probabilities::InclusionProbabilities;
use crate::support::caching::Cache;
use nym_api_requests::models::{GatewayBondAnnotated, MixNodeBondAnnotated, MixnodeStatus};
use nym_contracts_common::{IdentityKey, IdentityKeyRef};
use nym_mixnet_contract_common::MixId;
use rocket::fairing::AdHoc;
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::sync::RwLockReadGuard;
use tokio::{sync::RwLock, time};

const CACHE_TIMEOUT_MS: u64 = 100;

pub mod data;
mod inclusion_probabilities;
mod node_sets;
pub mod refresher;

#[derive(Debug, Error)]
enum NodeStatusCacheError {
    #[error("failed to simulate selection probabilities for mixnodes, not updating cache")]
    SimulationFailed,

    #[error("the current interval information is not available at the moment")]
    SourceDataMissing,
}

/// A node status cache suitable for caching values computed in one sweep, such as active set
/// inclusion probabilities that are computed for all mixnodes at the same time.
///
/// The cache can be triggered to update on contract cache changes, and/or periodically on a timer.
#[derive(Clone)]
pub struct NodeStatusCache {
    inner: Arc<RwLock<NodeStatusCacheData>>,
}

impl NodeStatusCache {
    /// Creates a new cache with no data.
    fn new() -> NodeStatusCache {
        NodeStatusCache {
            inner: Arc::new(RwLock::new(NodeStatusCacheData::new())),
        }
    }

    pub fn stage() -> AdHoc {
        AdHoc::on_ignite("Node Status Cache", |rocket| async {
            rocket.manage(Self::new())
        })
    }

    /// Updates the cache with the latest data.
    async fn update(
        &self,
        mixnodes: HashMap<MixId, MixNodeBondAnnotated>,
        rewarded_set: Vec<MixNodeBondAnnotated>,
        active_set: Vec<MixNodeBondAnnotated>,
        gateways: HashMap<IdentityKey, GatewayBondAnnotated>,
        inclusion_probabilities: InclusionProbabilities,
    ) {
        match time::timeout(Duration::from_millis(CACHE_TIMEOUT_MS), self.inner.write()).await {
            Ok(mut cache) => {
                cache.mixnodes_annotated.unchecked_update(mixnodes);
                cache.rewarded_set_annotated.unchecked_update(rewarded_set);
                cache.active_set_annotated.unchecked_update(active_set);
                cache.gateways_annotated.unchecked_update(gateways);
                cache
                    .inclusion_probabilities
                    .unchecked_update(inclusion_probabilities);
            }
            Err(e) => error!("{e}"),
        }
    }

    /// Returns a copy of the current cache data.
    async fn get_owned<T>(
        &self,
        fn_arg: impl FnOnce(RwLockReadGuard<'_, NodeStatusCacheData>) -> Cache<T>,
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
        fn_arg: impl FnOnce(&NodeStatusCacheData) -> &Cache<T>,
    ) -> Option<RwLockReadGuard<'a, Cache<T>>> {
        match time::timeout(Duration::from_millis(CACHE_TIMEOUT_MS), self.inner.read()).await {
            Ok(cache) => Some(RwLockReadGuard::map(cache, |item| fn_arg(item))),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }

    pub(crate) async fn active_mixnodes_cache(
        &self,
    ) -> Option<RwLockReadGuard<Cache<Vec<MixNodeBondAnnotated>>>> {
        self.get(|c| &c.active_set_annotated).await
    }

    pub(crate) async fn mixnodes_annotated_full(&self) -> Option<Vec<MixNodeBondAnnotated>> {
        let mixnodes = self.get(|c| &c.mixnodes_annotated).await?;

        // just clone everything and return the vec to work with the existing code
        Some(mixnodes.values().cloned().collect())
    }

    pub(crate) async fn mixnodes_annotated_filtered(&self) -> Option<Vec<MixNodeBondAnnotated>> {
        let full = self.mixnodes_annotated_full().await?;
        Some(full.iter().filter(|m| !m.blacklisted).cloned().collect())
    }

    pub(crate) async fn mixnode_annotated(&self, mix_id: MixId) -> Option<MixNodeBondAnnotated> {
        let mixnodes = self.get(|c| &c.mixnodes_annotated).await?;
        mixnodes.get(&mix_id).cloned()
    }

    pub(crate) async fn rewarded_set_annotated(&self) -> Option<Cache<Vec<MixNodeBondAnnotated>>> {
        self.get_owned(|c| c.rewarded_set_annotated.clone_cache())
            .await
    }

    pub(crate) async fn active_set_annotated(&self) -> Option<Cache<Vec<MixNodeBondAnnotated>>> {
        self.get_owned(|c| c.active_set_annotated.clone_cache())
            .await
    }

    pub(crate) async fn gateways_cache(
        &self,
    ) -> Option<RwLockReadGuard<Cache<HashMap<IdentityKey, GatewayBondAnnotated>>>> {
        self.get(|c| &c.gateways_annotated).await
    }

    pub(crate) async fn gateways_annotated_full(&self) -> Option<Vec<GatewayBondAnnotated>> {
        let gateways = self.get(|c| &c.gateways_annotated).await?;

        // just clone everything and return the vec to work with the existing code
        Some(gateways.values().cloned().collect())
    }

    pub(crate) async fn gateways_annotated_filtered(&self) -> Option<Vec<GatewayBondAnnotated>> {
        let full = self.gateways_annotated_full().await?;
        Some(full.iter().filter(|m| !m.blacklisted).cloned().collect())
    }

    pub(crate) async fn gateway_annotated(
        &self,
        gateway_id: IdentityKeyRef<'_>,
    ) -> Option<GatewayBondAnnotated> {
        let gateways = self.get(|c| &c.gateways_annotated).await?;
        gateways.get(gateway_id).cloned()
    }

    pub(crate) async fn inclusion_probabilities(&self) -> Option<Cache<InclusionProbabilities>> {
        self.get_owned(|c| c.inclusion_probabilities.clone_cache())
            .await
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

        let all_bonded = &self.mixnodes_annotated_filtered().await.unwrap();
        if let Some(bond) = all_bonded.iter().find(|mix| mix.mix_id() == mix_id) {
            (Some(bond.clone()), MixnodeStatus::Inactive)
        } else {
            (None, MixnodeStatus::NotFound)
        }
    }
}
