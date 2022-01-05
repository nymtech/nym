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
}

impl MixNodesResult {
    fn new() -> Self {
        MixNodesResult {
            valid_until: SystemTime::now() - Duration::from_secs(60), // in the past
            all_mixnodes: HashMap::new(),
            active_mixnodes: HashSet::new(),
            rewarded_mixnodes: HashSet::new(),
        }
    }

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
    mixnode_results: Arc<RwLock<MixNodesResult>>,
    location_cache: Arc<RwLock<LocationCache>>,
}

impl ThreadsafeMixNodesResult {
    pub(crate) fn new() -> Self {
        ThreadsafeMixNodesResult {
            mixnode_results: Arc::new(RwLock::new(MixNodesResult::new())),
            location_cache: Arc::new(RwLock::new(LocationCache::new())),
        }
    }

    pub(crate) fn new_with_location_cache(location_cache: LocationCache) -> Self {
        ThreadsafeMixNodesResult {
            mixnode_results: Arc::new(RwLock::new(MixNodesResult::new())),
            location_cache: Arc::new(RwLock::new(location_cache)),
        }
    }

    pub(crate) async fn is_location_valid(&self, identity_key: &str) -> bool {
        self.location_cache
            .read()
            .await
            .get(identity_key)
            .map(|cache_item| cache_item.valid_until > SystemTime::now())
            .unwrap_or(false)
    }

    pub(crate) async fn get_location_cache(&self) -> LocationCache {
        self.location_cache.read().await.clone()
    }

    pub(crate) async fn set_location(&self, identity_key: &str, location: Option<Location>) {
        // cache the location for this mix node so that it can be used when the mix node list is refreshed
        self.location_cache.write().await.insert(
            identity_key.to_string(),
            LocationCacheItem::new_from_location(location),
        );
    }

    async fn is_valid(&self) -> bool {
        self.mixnode_results.read().await.valid_until >= SystemTime::now()
    }

    pub(crate) async fn get_mixnode(&self, pubkey: &str) -> Option<MixNodeBond> {
        if self.is_valid().await {
            self.mixnode_results
                .read()
                .await
                .all_mixnodes
                .get(pubkey)
                .cloned()
        } else {
            None
        }
    }

    pub(crate) async fn get_mixnodes(&self) -> Option<HashMap<String, MixNodeBond>> {
        if self.is_valid().await {
            Some(self.mixnode_results.read().await.all_mixnodes.clone())
        } else {
            None
        }
    }

    pub(crate) async fn get_detailed_mixnodes(&self) -> Vec<PrettyDetailedMixNodeBond> {
        let mixnodes_guard = self.mixnode_results.read().await;
        let location_guard = self.location_cache.read().await;

        mixnodes_guard
            .all_mixnodes
            .values()
            .map(|bond| {
                let location = location_guard.get(&bond.mix_node.identity_key);
                let copy = bond.clone();
                PrettyDetailedMixNodeBond {
                    location: location.and_then(|l| l.location.clone()),
                    status: mixnodes_guard.determine_node_status(&bond.mix_node.identity_key),
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
        let mut guard = self.mixnode_results.write().await;
        guard.all_mixnodes = all_bonds
            .into_iter()
            .map(|bond| (bond.mix_node.identity_key.to_string(), bond))
            .collect();
        guard.rewarded_mixnodes = rewarded_nodes;
        guard.active_mixnodes = active_nodes;
        guard.valid_until = SystemTime::now() + MIXNODES_CACHE_ENTRY_TTL;
    }
}
