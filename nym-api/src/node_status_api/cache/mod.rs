// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::caching::Cache;

use self::data::NodeStatusCacheData;
use self::inclusion_probabilities::InclusionProbabilities;
use mixnet_contract_common::MixId;
use nym_api_requests::models::{MixNodeBondAnnotated, MixnodeStatus};
use rocket::fairing::AdHoc;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLockReadGuard;
use tokio::{sync::RwLock, time};

const CACHE_TIMEOUT_MS: u64 = 100;

pub mod data;
mod inclusion_probabilities;
pub mod refresher;

enum NodeStatusCacheError {
    SimulationFailed,
    SourceDataMissing,
}

// A node status cache suitable for caching values computed in one sweep, such as active set
// inclusion probabilities that are computed for all mixnodes at the same time.
//
// The cache can be triggered to update on contract cache changes, and/or periodically on a timer.
#[derive(Clone)]
pub struct NodeStatusCache {
    inner: Arc<RwLock<NodeStatusCacheData>>,
}

impl NodeStatusCache {
    fn new() -> Self {
        NodeStatusCache {
            inner: Arc::new(RwLock::new(NodeStatusCacheData::new())),
        }
    }

    pub fn stage() -> AdHoc {
        AdHoc::on_ignite("Node Status Cache", |rocket| async {
            rocket.manage(Self::new())
        })
    }

    async fn update_cache(
        &self,
        mixnodes: Vec<MixNodeBondAnnotated>,
        rewarded_set: Vec<MixNodeBondAnnotated>,
        active_set: Vec<MixNodeBondAnnotated>,
        inclusion_probabilities: InclusionProbabilities,
    ) {
        match time::timeout(Duration::from_millis(CACHE_TIMEOUT_MS), self.inner.write()).await {
            Ok(mut cache) => {
                cache.mixnodes_annotated.update(mixnodes);
                cache.rewarded_set_annotated.update(rewarded_set);
                cache.active_set_annotated.update(active_set);
                cache
                    .inclusion_probabilities
                    .update(inclusion_probabilities);
            }
            Err(e) => error!("{e}"),
        }
    }

    async fn get_cache<T>(
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

    pub(crate) async fn mixnodes_annotated(&self) -> Option<Cache<Vec<MixNodeBondAnnotated>>> {
        self.get_cache(|c| c.mixnodes_annotated.clone()).await
    }

    pub(crate) async fn rewarded_set_annotated(&self) -> Option<Cache<Vec<MixNodeBondAnnotated>>> {
        self.get_cache(|c| c.rewarded_set_annotated.clone()).await
    }

    pub(crate) async fn active_set_annotated(&self) -> Option<Cache<Vec<MixNodeBondAnnotated>>> {
        self.get_cache(|c| c.active_set_annotated.clone()).await
    }

    pub(crate) async fn inclusion_probabilities(&self) -> Option<Cache<InclusionProbabilities>> {
        self.get_cache(|c| c.inclusion_probabilities.clone()).await
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

        let all_bonded = &self.mixnodes_annotated().await.unwrap().into_inner();
        if let Some(bond) = all_bonded.iter().find(|mix| mix.mix_id() == mix_id) {
            (Some(bond.clone()), MixnodeStatus::Inactive)
        } else {
            (None, MixnodeStatus::NotFound)
        }
    }
}
