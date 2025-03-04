// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::location::{LocationCache, LocationCacheItem};
use crate::unstable::location::NymNodeLocationCache;
use crate::unstable::CACHE_ENTRY_TTL;
use nym_explorer_api_requests::{
    Location, NymNodeWithDescriptionAndLocation, PrettyDetailedGatewayBond,
};
use nym_mixnet_contract_common::{Gateway, NodeId, NymNodeDetails};
use nym_validator_client::models::{NymNodeData, NymNodeDescription};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, RwLockReadGuard};

pub(crate) struct NymNodesCache {
    pub(crate) valid_until: SystemTime,
    pub(crate) bonded_nym_nodes: HashMap<NodeId, NymNodeDetails>,
    pub(crate) described_nodes: HashMap<NodeId, NymNodeDescription>,
}

impl NymNodesCache {
    fn new() -> Self {
        NymNodesCache {
            valid_until: SystemTime::now() - Duration::from_secs(60), // in the past
            bonded_nym_nodes: Default::default(),
            described_nodes: Default::default(),
        }
    }

    // fn is_valid(&self) -> bool {
    //     self.valid_until >= SystemTime::now()
    // }
}

#[derive(Clone)]
pub(crate) struct ThreadSafeNymNodesCache {
    nymnodes: Arc<RwLock<NymNodesCache>>,
    locations: Arc<RwLock<LocationCache<NodeId>>>,
}

impl ThreadSafeNymNodesCache {
    pub(crate) fn new() -> Self {
        ThreadSafeNymNodesCache {
            nymnodes: Arc::new(RwLock::new(NymNodesCache::new())),
            locations: Arc::new(RwLock::new(NymNodeLocationCache::new())),
        }
    }

    pub(crate) fn new_with_location_cache(locations: NymNodeLocationCache) -> Self {
        ThreadSafeNymNodesCache {
            nymnodes: Arc::new(RwLock::new(NymNodesCache::new())),
            locations: Arc::new(RwLock::new(locations)),
        }
    }

    pub(crate) async fn is_location_valid(&self, node_id: NodeId) -> bool {
        self.locations
            .read()
            .await
            .get(&node_id)
            .is_some_and(|cache_item| cache_item.valid_until > SystemTime::now())
    }

    pub(crate) async fn get_bonded_nymnodes(
        &self,
    ) -> RwLockReadGuard<HashMap<NodeId, NymNodeDetails>> {
        let guard = self.nymnodes.read().await;
        RwLockReadGuard::map(guard, |n| &n.bonded_nym_nodes)
    }

    pub(crate) async fn get_bonded_nymnodes_descriptions(&self) -> Vec<NymNodeData> {
        let guard = self.nymnodes.read().await;
        guard
            .described_nodes
            .values()
            .map(|i| i.description.clone())
            .collect()
    }

    pub(crate) async fn get_bonded_nymnodes_locations(&self) -> Vec<Location> {
        let guard_locations = self.locations.read().await;
        let mut locations: Vec<Location> = vec![];
        for location in guard_locations.values() {
            if let Some(l) = &location.location {
                locations.push(l.clone());
            }
        }
        locations
    }

    pub(crate) async fn get_bonded_nymnodes_with_description_and_location(
        &self,
    ) -> HashMap<NodeId, NymNodeWithDescriptionAndLocation> {
        let guard_nodes = self.nymnodes.read().await;
        let guard_locations = self.locations.read().await;

        let mut map: HashMap<NodeId, NymNodeWithDescriptionAndLocation> = HashMap::new();

        for (node_id, node) in guard_nodes.bonded_nym_nodes.clone() {
            let description = guard_nodes.described_nodes.get(&node_id);
            let location = guard_locations.get(&node_id);

            map.insert(
                node_id,
                NymNodeWithDescriptionAndLocation {
                    node_id,
                    description: description.map(|d| d.description.clone()),
                    location: location.and_then(|l| l.location.clone()),
                    contract_node_type: description.map(|d| d.contract_node_type),
                    bond_information: node.bond_information,
                    rewarding_details: node.rewarding_details,
                },
            );
        }

        map
    }

    pub(crate) async fn get_locations(&self) -> NymNodeLocationCache {
        self.locations.read().await.clone()
    }

    pub(crate) async fn set_location(&self, node_id: NodeId, location: Option<Location>) {
        // cache the location for this mix node so that it can be used when the mix node list is refreshed
        self.locations
            .write()
            .await
            .insert(node_id, LocationCacheItem::new_from_location(location));
    }

    pub(crate) async fn update_cache(
        &self,
        all_bonds: Vec<NymNodeDetails>,
        descriptions: Vec<NymNodeDescription>,
    ) {
        let mut guard = self.nymnodes.write().await;
        guard.bonded_nym_nodes = all_bonds
            .into_iter()
            .map(|details| (details.node_id(), details))
            .collect();
        guard.described_nodes = descriptions
            .into_iter()
            .map(|description| (description.node_id, description))
            .collect();

        guard.valid_until = SystemTime::now() + CACHE_ENTRY_TTL;
    }

    pub(crate) async fn pretty_gateways(&self) -> Vec<PrettyDetailedGatewayBond> {
        let nodes_guard = self.nymnodes.read().await;
        let location_guard = self.locations.read().await;

        let mut pretty_gateways = vec![];

        for (node_id, native_nymnode) in &nodes_guard.bonded_nym_nodes {
            let Some(description) = nodes_guard.described_nodes.get(node_id) else {
                continue;
            };

            if description.description.declared_role.entry {
                let location = location_guard.get(node_id);
                let bond = &native_nymnode.bond_information;

                pretty_gateways.push(PrettyDetailedGatewayBond {
                    pledge_amount: bond.original_pledge.clone(),
                    owner: bond.owner.clone(),
                    block_height: bond.bonding_height,
                    gateway: Gateway {
                        host: bond.node.host.clone(),
                        mix_port: description.description.mix_port(),
                        clients_port: description.description.mixnet_websockets.ws_port,
                        location: description
                            .description
                            .auxiliary_details
                            .location
                            .as_ref()
                            .map(|l| l.to_string())
                            .unwrap_or_default(),
                        sphinx_key: description
                            .description
                            .host_information
                            .keys
                            .x25519
                            .to_base58_string(),
                        identity_key: bond.node.identity_key.clone(),
                        version: description
                            .description
                            .build_information
                            .build_version
                            .clone(),
                    },
                    proxy: None,
                    location: location.and_then(|l| l.location.clone()),
                })
            }
        }

        pretty_gateways
    }
}
