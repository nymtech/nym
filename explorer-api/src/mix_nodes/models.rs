// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tokio::sync::RwLock;

use validator_client::models::{MixNodeBondAnnotated, UptimeResponse};

use crate::cache::Cache;
use crate::mix_node::models::{MixnodeStatus, PrettyDetailedMixNodeBond};
use crate::mix_nodes::location::{Location, LocationCache, LocationCacheItem};
use crate::mix_nodes::CACHE_ENTRY_TTL;

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct MixNodeActiveSetSummary {
    pub active: usize,
    pub standby: usize,
    pub inactive: usize,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct MixNodeSummary {
    pub count: usize,
    pub activeset: MixNodeActiveSetSummary,
}

#[derive(Clone, Debug)]
pub(crate) struct MixNodesResult {
    pub(crate) valid_until: SystemTime,
    pub(crate) all_mixnodes: HashMap<String, MixNodeBondAnnotated>,
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

    fn is_valid(&self) -> bool {
        self.valid_until >= SystemTime::now()
    }

    fn get_mixnode(&self, pubkey: &str) -> Option<MixNodeBondAnnotated> {
        if self.is_valid() {
            self.all_mixnodes.get(pubkey).cloned()
        } else {
            None
        }
    }

    fn get_mixnodes(&self) -> Option<HashMap<String, MixNodeBondAnnotated>> {
        if self.is_valid() {
            Some(self.all_mixnodes.clone())
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MixNodeHealth {
    avg_uptime: u8,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeMixNodesCache {
    mixnodes: Arc<RwLock<MixNodesResult>>,
    locations: Arc<RwLock<LocationCache>>,
    mixnode_health: Arc<RwLock<Cache<MixNodeHealth>>>,
}

impl ThreadsafeMixNodesCache {
    pub(crate) fn new() -> Self {
        ThreadsafeMixNodesCache {
            mixnodes: Arc::new(RwLock::new(MixNodesResult::new())),
            locations: Arc::new(RwLock::new(LocationCache::new())),
            mixnode_health: Arc::new(RwLock::new(Cache::new())),
        }
    }

    pub(crate) fn new_with_location_cache(locations: LocationCache) -> Self {
        ThreadsafeMixNodesCache {
            mixnodes: Arc::new(RwLock::new(MixNodesResult::new())),
            locations: Arc::new(RwLock::new(locations)),
            mixnode_health: Arc::new(RwLock::new(Cache::new())),
        }
    }

    pub(crate) async fn is_location_valid(&self, identity_key: &str) -> bool {
        self.locations
            .read()
            .await
            .get(identity_key)
            .map_or(false, |cache_item| cache_item.valid_until > SystemTime::now())
    }

    pub(crate) async fn get_locations(&self) -> LocationCache {
        self.locations.read().await.clone()
    }

    pub(crate) async fn set_location(&self, identity_key: &str, location: Option<Location>) {
        // cache the location for this mix node so that it can be used when the mix node list is refreshed
        self.locations.write().await.insert(
            identity_key.to_string(),
            LocationCacheItem::new_from_location(location),
        );
    }

    pub(crate) async fn get_mixnode(&self, pubkey: &str) -> Option<MixNodeBondAnnotated> {
        self.mixnodes.read().await.get_mixnode(pubkey)
    }

    pub(crate) async fn get_mixnodes(&self) -> Option<HashMap<String, MixNodeBondAnnotated>> {
        self.mixnodes.read().await.get_mixnodes()
    }

    pub(crate) async fn get_detailed_mixnode_by_id(
        &self,
        identity_key: &str,
    ) -> Option<PrettyDetailedMixNodeBond> {
        let mixnodes_guard = self.mixnodes.read().await;
        let location_guard = self.locations.read().await;
        let mixnode_health_guard = self.mixnode_health.read().await;

        let bond = mixnodes_guard.get_mixnode(identity_key);
        let location = location_guard.get(identity_key);
        let health = mixnode_health_guard.get(identity_key);

        match bond {
            Some(bond) => Some(PrettyDetailedMixNodeBond {
                location: location.and_then(|l| l.location.clone()),
                status: mixnodes_guard.determine_node_status(&bond.mix_node().identity_key),
                pledge_amount: bond.mixnode_bond.pledge_amount,
                total_delegation: bond.mixnode_bond.total_delegation,
                owner: bond.mixnode_bond.owner,
                layer: bond.mixnode_bond.layer,
                mix_node: bond.mixnode_bond.mix_node,
                avg_uptime: health.map(|m| m.avg_uptime),
                stake_saturation: bond.stake_saturation,
                estimated_operator_apy: bond.estimated_operator_apy,
                estimated_delegators_apy: bond.estimated_delegators_apy,
            }),
            None => None,
        }
    }

    pub(crate) async fn get_detailed_mixnodes(&self) -> Vec<PrettyDetailedMixNodeBond> {
        let mixnodes_guard = self.mixnodes.read().await;
        let location_guard = self.locations.read().await;
        let mixnode_health_guard = self.mixnode_health.read().await;

        mixnodes_guard
            .all_mixnodes
            .values()
            .map(|bond| {
                let location = location_guard.get(&bond.mix_node().identity_key);
                let copy = bond.mixnode_bond.clone();
                let health = mixnode_health_guard.get(&bond.mix_node().identity_key);
                PrettyDetailedMixNodeBond {
                    location: location.and_then(|l| l.location.clone()),
                    status: mixnodes_guard.determine_node_status(&bond.mix_node().identity_key),
                    pledge_amount: copy.pledge_amount,
                    total_delegation: copy.total_delegation,
                    owner: copy.owner,
                    layer: copy.layer,
                    mix_node: copy.mix_node,
                    avg_uptime: health.map(|m| m.avg_uptime),
                    stake_saturation: bond.stake_saturation,
                    estimated_operator_apy: bond.estimated_operator_apy,
                    estimated_delegators_apy: bond.estimated_delegators_apy,
                }
            })
            .collect()
    }

    pub(crate) async fn update_cache(
        &self,
        all_bonds: Vec<MixNodeBondAnnotated>,
        rewarded_nodes: HashSet<String>,
        active_nodes: HashSet<String>,
    ) {
        let mut guard = self.mixnodes.write().await;
        guard.all_mixnodes = all_bonds
            .into_iter()
            .map(|bond| (bond.mix_node().identity_key.to_string(), bond))
            .collect();
        guard.rewarded_mixnodes = rewarded_nodes;
        guard.active_mixnodes = active_nodes;
        guard.valid_until = SystemTime::now() + CACHE_ENTRY_TTL;
    }

    pub(crate) async fn update_health_cache(&self, all_uptimes: Vec<UptimeResponse>) {
        let mut mixnode_health = self.mixnode_health.write().await;
        for uptime in all_uptimes {
            let health = MixNodeHealth {
                avg_uptime: uptime.avg_uptime,
            };
            mixnode_health.set(&uptime.identity, health);
        }
    }
}
