// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_node::models::{MixnodeStatus, PrettyDetailedMixNodeBond};
use crate::mix_nodes::location::{Location, LocationCache, LocationCacheItem};
use crate::mix_nodes::MIXNODES_CACHE_ENTRY_TTL;
use mixnet_contract::MixNodeBond;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub(crate) struct MixNodesResult {
    pub(crate) valid_until: SystemTime,
    pub(crate) all_mixnodes: HashMap<String, MixNodeBond>,
    active_mixnodes: HashSet<String>,
    rewarded_mixnodes: HashSet<String>,
    location_cache: LocationCache,
}

impl MixNodesResult {
    fn determine_node_status(&self, public_key: &str) -> MixnodeStatus {
        if self.active_mixnodes.contains(public_key) {
            MixnodeStatus::Active
        } else if self.rewarded_mixnodes.contains(public_key) {
            MixnodeStatus::Standby
        } else {
            MixnodeStatus::Inactive
        }
    }
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
                active_mixnodes: HashSet::new(),
                rewarded_mixnodes: HashSet::new(),
                location_cache: LocationCache::new(),
            })),
        }
    }

    pub(crate) fn new_with_location_cache(location_cache: LocationCache) -> Self {
        ThreadsafeMixNodesResult {
            inner: Arc::new(RwLock::new(MixNodesResult {
                valid_until: SystemTime::now() - Duration::from_secs(60), // in the past
                all_mixnodes: HashMap::new(),
                active_mixnodes: HashSet::new(),
                rewarded_mixnodes: HashSet::new(),
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

    async fn is_valid(&self) -> bool {
        self.inner.read().await.valid_until >= SystemTime::now()
    }

    pub(crate) async fn get_mixnode(&self, pubkey: &str) -> Option<MixNodeBond> {
        if self.is_valid().await {
            self.inner.read().await.all_mixnodes.get(pubkey).cloned()
        } else {
            None
        }
    }

    pub(crate) async fn get_mixnodes(&self) -> Option<HashMap<String, MixNodeBond>> {
        if self.is_valid().await {
            Some(self.inner.read().await.all_mixnodes.clone())
        } else {
            None
        }
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
                    status: guard.determine_node_status(&bond.mix_node.identity_key),
                    pledge_amount: copy.pledge_amount,
                    total_delegation: copy.total_delegation,
                    owner: copy.owner,
                    layer: copy.layer,
                    mix_node: copy.mix_node,
                }
            })
            .collect()
    }

    pub(crate) async fn update_cache(
        &self,
        all_bonds: Vec<MixNodeBond>,
        rewarded_nodes: HashSet<String>,
        active_nodes: HashSet<String>,
    ) {
        let mut guard = self.inner.write().await;
        guard.all_mixnodes = all_bonds
            .into_iter()
            .map(|bond| (bond.mix_node.identity_key.to_string(), bond))
            .collect();
        guard.rewarded_mixnodes = rewarded_nodes;
        guard.active_mixnodes = active_nodes;
        guard.valid_until = SystemTime::now() + MIXNODES_CACHE_ENTRY_TTL;
    }
}
