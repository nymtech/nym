// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_node::models::PrettyDetailedMixNodeBond;
use crate::mix_nodes::location::{Location, LocationCache, LocationCacheItem};
use mixnet_contract::MixNodeBond;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub(crate) struct MixNodesResult {
    pub(crate) valid_until: SystemTime,
    pub(crate) all_mixnodes: HashMap<String, MixNodeBond>,
    // active_mixnodes: HashSet<String>,
    // rewarded_mixnodes: HashSet<String>,
    location_cache: LocationCache,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeMixNodesResult {
    inner: Arc<RwLock<MixNodesResult>>,
}

impl ThreadsafeMixNodesResult {
    pub(crate) fn new() -> Self {
        ThreadsafeMixNodesResult {
            inner: Arc::new(RwLock::new(MixNodesResult {
                valid_until: SystemTime::now() - Duration::from_secs(60), // in the past
                all_mixnodes: HashMap::new(),
                // active_mixnodes: Default::default(),
                // rewarded_mixnodes: Default::default(),
                location_cache: LocationCache::new(),
            })),
        }
    }

    pub(crate) fn new_with_location_cache(location_cache: LocationCache) -> Self {
        ThreadsafeMixNodesResult {
            inner: Arc::new(RwLock::new(MixNodesResult {
                valid_until: SystemTime::now() - Duration::from_secs(60), // in the past
                all_mixnodes: HashMap::new(),
                // active_mixnodes: Default::default(),
                // rewarded_mixnodes: Default::default(),
                location_cache,
            })),
        }
    }

    pub(crate) async fn is_location_valid(&self, identity_key: &str) -> bool {
        self.inner
            .read()
            .await
            .location_cache
            .get(identity_key)
            .map(|cache_item| cache_item.valid_until > SystemTime::now())
            .unwrap_or(false)
    }

    pub(crate) async fn get_location_cache(&self) -> LocationCache {
        self.inner.read().await.location_cache.clone()
    }

    pub(crate) async fn set_location(&self, identity_key: &str, location: Option<Location>) {
        let mut guard = self.inner.write().await;

        // cache the location for this mix node so that it can be used when the mix node list is refreshed
        guard.location_cache.insert(
            identity_key.to_string(),
            LocationCacheItem::new_from_location(location),
        );
    }

    pub(crate) async fn get(&self) -> MixNodesResult {
        // check ttl
        let valid_until = self.inner.read().await.valid_until;

        if valid_until < SystemTime::now() {
            // force reload
            todo!()
            // self.update_cache().await;
        }

        // return in-memory cache
        self.inner.read().await.clone()
    }

    pub(crate) async fn get_detailed_mixnodes(&self) -> Vec<PrettyDetailedMixNodeBond> {
        let guard = self.inner.read().await;
        guard
            .all_mixnodes
            .values()
            .map(|bond| {
                let location = guard.location_cache.get(&bond.mix_node.identity_key);
                let copy = bond.clone();
                PrettyDetailedMixNodeBond {
                    location: location.and_then(|l| l.location.clone()),
                    // status: MixnodeStatus::Active,
                    pledge_amount: copy.pledge_amount,
                    total_delegation: copy.total_delegation,
                    owner: copy.owner,
                    layer: copy.layer,
                    mix_node: copy.mix_node,
                }
            })
            .collect()
    }

    pub(crate) async fn update_cache(&self, bonds: Vec<MixNodeBond>) {
        let mut guard = self.inner.write().await;
        guard.all_mixnodes = bonds
            .into_iter()
            .map(|bond| (bond.mix_node.identity_key.to_string(), bond))
            .collect();

        // TODO: update active/rewarded

        guard.valid_until = SystemTime::now() + Duration::from_secs(30);
    }
}
