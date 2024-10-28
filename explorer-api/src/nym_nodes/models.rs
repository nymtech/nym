// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::location::{LocationCache, LocationCacheItem};
use crate::nym_nodes::location::NymNodeLocationCache;
use crate::nym_nodes::CACHE_ENTRY_TTL;
use nym_explorer_api_requests::{
    Location, MixnodeStatus, PrettyDetailedGatewayBond, PrettyDetailedMixNodeBond,
};
use nym_mixnet_contract_common::{Gateway, LegacyMixLayer, MixNode, NodeId, NymNodeDetails};
use nym_network_defaults::DEFAULT_HTTP_API_LISTENING_PORT;
use nym_validator_client::models::NymNodeDescription;
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
            .map_or(false, |cache_item| {
                cache_item.valid_until > SystemTime::now()
            })
    }

    pub(crate) async fn get_bonded_nymnodes(
        &self,
    ) -> RwLockReadGuard<HashMap<NodeId, NymNodeDetails>> {
        let guard = self.nymnodes.read().await;
        RwLockReadGuard::map(guard, |n| &n.bonded_nym_nodes)
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

    pub(crate) async fn pretty_mixnodes(&self) -> Vec<PrettyDetailedMixNodeBond> {
        let nodes_guard = self.nymnodes.read().await;
        let location_guard = self.locations.read().await;

        let mut pretty_mixnodes = vec![];

        for (node_id, native_nymnode) in &nodes_guard.bonded_nym_nodes {
            let Some(description) = nodes_guard.described_nodes.get(node_id) else {
                continue;
            };

            if description.description.declared_role.entry {
                let location = location_guard.get(node_id);
                let bond = &native_nymnode.bond_information;

                pretty_mixnodes.push(PrettyDetailedMixNodeBond {
                    mix_id: *node_id,
                    pledge_amount: bond.original_pledge.clone(),
                    total_delegation: Default::default(),
                    owner: bond.owner.clone(),
                    layer: LegacyMixLayer::One,
                    mix_node: MixNode {
                        host: bond.node.host.clone(),
                        mix_port: description.description.mix_port(),
                        verloc_port: description.description.verloc_port(),
                        http_api_port: bond
                            .node
                            .custom_http_port
                            .unwrap_or(DEFAULT_HTTP_API_LISTENING_PORT),
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
                    stake_saturation: 0.0,
                    uncapped_saturation: 0.0,
                    avg_uptime: 0,
                    node_performance: Default::default(),
                    estimated_operator_apy: 0.0,
                    estimated_delegators_apy: 0.0,
                    operating_cost: native_nymnode
                        .rewarding_details
                        .cost_params
                        .interval_operating_cost
                        .clone(),
                    profit_margin_percent: native_nymnode
                        .rewarding_details
                        .cost_params
                        .profit_margin_percent,
                    family_id: None,
                    location: location.and_then(|l| l.location.clone()),
                    status: MixnodeStatus::Inactive,
                    blacklisted: true,
                })
            }
        }

        pretty_mixnodes
    }
}
