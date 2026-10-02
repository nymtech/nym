// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::shared_network::CachedNetwork;
use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519;
use nym_topology::{
    EntryDetails, LewesProtocolDetailsDataV1, NodeId, NymTopology, Role, RoutingNode,
    TopologyProvider,
};
use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use std::sync::Arc;
use tracing::debug;

const LOCAL_NODE_ID: NodeId = 1234567890;

pub(crate) struct LocalGatewayNode {
    pub(crate) active_sphinx_keys: ActiveSphinxKeys,
    pub(crate) mix_host: SocketAddr,
    pub(crate) lp_data_host: SocketAddr,
    pub(crate) ip_addresses: Vec<IpAddr>,
    pub(crate) identity_key: ed25519::PublicKey,
    pub(crate) entry: EntryDetails,
    pub(crate) lp: LewesProtocolDetailsDataV1,
    pub(crate) build_version: semver::Version,
}

impl LocalGatewayNode {
    pub(crate) fn to_routing_node(&self) -> RoutingNode {
        RoutingNode {
            node_id: LOCAL_NODE_ID,
            mix_host: self.mix_host,
            lp_data_host: self.lp_data_host,
            ip_addresses: self.ip_addresses.clone(),
            entry: Some(self.entry.clone()),
            identity_key: self.identity_key,
            sphinx_key: self.active_sphinx_keys.primary().deref().x25519_pubkey(),
            supported_roles: nym_topology::SupportedRoles {
                mixnode: false,
                mixnet_entry: true,
                mixnet_exit: true,
            },
            // this is the node's own entry, so these are its own published details - the clients
            // embedded inside it reach it in-process rather than over any of these listeners
            lp: self.lp.clone(),
            build_version: self.build_version.clone(),
        }
    }
}

#[derive(Clone)]
pub struct CachedTopologyProvider {
    gateway_node: Arc<LocalGatewayNode>,
    cached_network: CachedNetwork,
    min_mix_performance: u8,
}

impl CachedTopologyProvider {
    pub(crate) fn new(
        gateway_node: LocalGatewayNode,
        cached_network: CachedNetwork,
        min_mix_performance: u8,
    ) -> Self {
        CachedTopologyProvider {
            gateway_node: Arc::new(gateway_node),
            cached_network,
            min_mix_performance,
        }
    }
}

#[async_trait]
impl TopologyProvider for CachedTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        let self_node = self.gateway_node.identity_key;

        let mut topology = self
            .cached_network
            .network_topology(self.min_mix_performance);

        if !topology.has_node(self.gateway_node.identity_key) {
            debug!("{self_node} didn't exist in topology. inserting it.",);
            topology.insert_node_details(self.gateway_node.to_routing_node());
        }
        topology.force_set_active(LOCAL_NODE_ID, Role::EntryGateway);

        Some(topology)
    }
}
