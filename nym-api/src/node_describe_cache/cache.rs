// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::described::v3::{
    DescribedNodeTypeV3, NymNodeDataV3, NymNodeDescriptionV3,
};
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribedNodes {
    pub(crate) nodes: HashMap<NodeId, NymNodeDescriptionV3>,
    pub(crate) addresses_cache: HashMap<IpAddr, NodeId>,
}

impl DescribedNodes {
    pub fn force_update(&mut self, node: NymNodeDescriptionV3) {
        for ip in &node.description.host_information.ip_address {
            self.addresses_cache.insert(*ip, node.node_id);
        }
        self.nodes.insert(node.node_id, node);
    }

    pub fn get_description(&self, node_id: &NodeId) -> Option<&NymNodeDataV3> {
        self.nodes.get(node_id).map(|n| &n.description)
    }

    pub fn get_node(&self, node_id: &NodeId) -> Option<&NymNodeDescriptionV3> {
        self.nodes.get(node_id)
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = &NymNodeDescriptionV3> {
        self.nodes.values()
    }

    pub fn all_nym_nodes(&self) -> impl Iterator<Item = &NymNodeDescriptionV3> {
        self.nodes
            .values()
            .filter(|n| n.contract_node_type == DescribedNodeTypeV3::NymNode)
    }

    pub fn mixing_nym_nodes(&self) -> impl Iterator<Item = &NymNodeDescriptionV3> {
        self.nodes
            .values()
            .filter(|n| n.contract_node_type == DescribedNodeTypeV3::NymNode)
            .filter(|n| n.description.declared_role.mixnode)
    }

    pub fn entry_capable_nym_nodes(&self) -> impl Iterator<Item = &NymNodeDescriptionV3> {
        self.nodes
            .values()
            .filter(|n| n.contract_node_type == DescribedNodeTypeV3::NymNode)
            .filter(|n| n.description.declared_role.entry)
    }

    pub fn exit_capable_nym_nodes(&self) -> impl Iterator<Item = &NymNodeDescriptionV3> {
        self.nodes
            .values()
            .filter(|n| n.contract_node_type == DescribedNodeTypeV3::NymNode)
            .filter(|n| n.description.declared_role.can_operate_exit_gateway())
    }

    pub fn node_with_address(&self, address: IpAddr) -> Option<NodeId> {
        self.addresses_cache.get(&address).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::caching::cache::test_helpers::round_trip_through_disk_cache;
    use nym_api_requests::models::described::v3::mock_nym_node_description;

    // This cache is persisted to disk with bincode; a populated value must survive that
    // round trip or the on-disk cache silently never writes.
    #[test]
    fn populated_cache_round_trips_through_the_on_disk_format() -> anyhow::Result<()> {
        let mut nodes = HashMap::new();
        let mut addresses_cache = HashMap::new();

        for seed in 0..3u64 {
            let mut node = mock_nym_node_description(seed);
            // the mock assigns a random node id; pin it so the assertions below are stable
            node.node_id = seed as NodeId + 1;
            addresses_cache.insert(IpAddr::from([127, 0, 0, seed as u8 + 1]), node.node_id);
            nodes.insert(node.node_id, node);
        }

        let described = DescribedNodes {
            nodes,
            addresses_cache,
        };

        let restored = round_trip_through_disk_cache(described)?;

        assert_eq!(restored.nodes.len(), 3);
        assert_eq!(restored.addresses_cache.len(), 3);
        assert!(restored.get_node(&1).is_some());
        assert_eq!(
            restored.node_with_address(IpAddr::from([127, 0, 0, 1])),
            Some(1)
        );
        Ok(())
    }
}
