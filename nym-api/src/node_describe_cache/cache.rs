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
