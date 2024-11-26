// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::location::MixnodeLocationCache;
use crate::helpers::best_effort_small_dec_to_f64;
use crate::location::LocationCacheItem;
use crate::mix_nodes::CACHE_ENTRY_TTL;
use nym_explorer_api_requests::{Location, MixnodeStatus, PrettyDetailedMixNodeBond};
use nym_mixnet_contract_common::rewarding::helpers::truncate_reward;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::models::MixNodeBondAnnotated;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, RwLockReadGuard};

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
    pub(crate) all_mixnodes: HashMap<NodeId, MixNodeBondAnnotated>,
    active_mixnodes: HashSet<NodeId>,
    rewarded_mixnodes: HashSet<NodeId>,
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

    fn determine_node_status(&self, mix_id: NodeId) -> MixnodeStatus {
        if self.active_mixnodes.contains(&mix_id) {
            MixnodeStatus::Active
        } else if self.rewarded_mixnodes.contains(&mix_id) {
            MixnodeStatus::Standby
        } else {
            MixnodeStatus::Inactive
        }
    }

    fn is_valid(&self) -> bool {
        self.valid_until >= SystemTime::now()
    }

    fn get_mixnode(&self, mix_id: NodeId) -> Option<MixNodeBondAnnotated> {
        if self.is_valid() {
            self.all_mixnodes.get(&mix_id).cloned()
        } else {
            None
        }
    }

    fn get_mixnodes(&self) -> Option<HashMap<NodeId, MixNodeBondAnnotated>> {
        if self.is_valid() {
            Some(self.all_mixnodes.clone())
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub(crate) struct ThreadsafeMixNodesCache {
    mixnodes: Arc<RwLock<MixNodesResult>>,
    locations: Arc<RwLock<MixnodeLocationCache>>,
}

impl ThreadsafeMixNodesCache {
    pub(crate) fn new() -> Self {
        ThreadsafeMixNodesCache {
            mixnodes: Arc::new(RwLock::new(MixNodesResult::new())),
            locations: Arc::new(RwLock::new(MixnodeLocationCache::new())),
        }
    }

    pub(crate) fn new_with_location_cache(locations: MixnodeLocationCache) -> Self {
        ThreadsafeMixNodesCache {
            mixnodes: Arc::new(RwLock::new(MixNodesResult::new())),
            locations: Arc::new(RwLock::new(locations)),
        }
    }

    pub(crate) async fn is_location_valid(&self, mix_id: NodeId) -> bool {
        self.locations
            .read()
            .await
            .get(&mix_id)
            .map_or(false, |cache_item| {
                cache_item.valid_until > SystemTime::now()
            })
    }

    pub(crate) async fn get_locations(&self) -> MixnodeLocationCache {
        self.locations.read().await.clone()
    }

    pub(crate) async fn set_location(&self, mix_id: NodeId, location: Option<Location>) {
        // cache the location for this mix node so that it can be used when the mix node list is refreshed
        self.locations
            .write()
            .await
            .insert(mix_id, LocationCacheItem::new_from_location(location));
    }

    pub(crate) async fn get_mixnode(&self, mix_id: NodeId) -> Option<MixNodeBondAnnotated> {
        self.mixnodes.read().await.get_mixnode(mix_id)
    }

    pub(crate) async fn get_mixnodes(&self) -> Option<HashMap<NodeId, MixNodeBondAnnotated>> {
        self.mixnodes.read().await.get_mixnodes()
    }

    fn create_detailed_mixnode(
        &self,
        mix_id: NodeId,
        mixnodes_guard: &RwLockReadGuard<'_, MixNodesResult>,
        location: Option<&LocationCacheItem>,
        node: &MixNodeBondAnnotated,
    ) -> PrettyDetailedMixNodeBond {
        let rewarding_info = &node.mixnode_details.rewarding_details;
        let denom = &rewarding_info.cost_params.interval_operating_cost.denom;

        PrettyDetailedMixNodeBond {
            mix_id,
            location: location.and_then(|l| l.location.clone()),
            status: mixnodes_guard.determine_node_status(mix_id),
            pledge_amount: truncate_reward(rewarding_info.operator, denom),
            total_delegation: truncate_reward(rewarding_info.delegates, denom),
            owner: node.mixnode_details.bond_information.owner.clone(),
            layer: node.mixnode_details.bond_information.layer,
            mix_node: node.mixnode_details.bond_information.mix_node.clone(),
            avg_uptime: node.performance.round_to_integer(),
            node_performance: node.node_performance.clone(),
            stake_saturation: best_effort_small_dec_to_f64(node.stake_saturation) as f32,
            uncapped_saturation: best_effort_small_dec_to_f64(node.uncapped_stake_saturation)
                as f32,
            estimated_operator_apy: best_effort_small_dec_to_f64(node.estimated_operator_apy),
            estimated_delegators_apy: best_effort_small_dec_to_f64(node.estimated_delegators_apy),
            operating_cost: rewarding_info.cost_params.interval_operating_cost.clone(),
            profit_margin_percent: rewarding_info.cost_params.profit_margin_percent,
            family_id: None,
            blacklisted: node.blacklisted,
        }
    }

    pub(crate) async fn get_detailed_mixnode(
        &self,
        mix_id: NodeId,
    ) -> Option<PrettyDetailedMixNodeBond> {
        let mixnodes_guard = self.mixnodes.read().await;
        let location_guard = self.locations.read().await;

        let bond = mixnodes_guard.get_mixnode(mix_id);
        let location = location_guard.get(&mix_id);

        bond.map(|bond| self.create_detailed_mixnode(mix_id, &mixnodes_guard, location, &bond))
    }

    pub(crate) async fn get_detailed_mixnodes(&self) -> Vec<PrettyDetailedMixNodeBond> {
        let mixnodes_guard = self.mixnodes.read().await;
        let location_guard = self.locations.read().await;

        mixnodes_guard
            .all_mixnodes
            .values()
            .map(|bond| {
                let location = location_guard.get(&bond.mix_id());
                self.create_detailed_mixnode(bond.mix_id(), &mixnodes_guard, location, bond)
            })
            .collect()
    }

    pub(crate) async fn update_cache(
        &self,
        all_bonds: Vec<MixNodeBondAnnotated>,
        rewarded_nodes: HashSet<NodeId>,
        active_nodes: HashSet<NodeId>,
    ) {
        let mut guard = self.mixnodes.write().await;
        guard.all_mixnodes = all_bonds
            .into_iter()
            .map(|bond| (bond.mix_id(), bond))
            .collect();
        guard.rewarded_mixnodes = rewarded_nodes;
        guard.active_mixnodes = active_nodes;
        guard.valid_until = SystemTime::now() + CACHE_ENTRY_TTL;
    }
}
