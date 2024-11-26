// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NodeId;
use nym_topology::{gateway, mix};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct TestableNode {
    pub encoded_identity: String,
    pub node_id: NodeId,

    #[serde(rename = "type")]
    pub typ: NodeType,
}

impl TestableNode {
    pub fn new(encoded_identity: String, typ: NodeType, node_id: NodeId) -> Self {
        TestableNode {
            encoded_identity,
            node_id,
            typ,
        }
    }

    pub fn new_mixnode(encoded_identity: String, node_id: NodeId) -> Self {
        TestableNode::new(encoded_identity, NodeType::Mixnode, node_id)
    }

    pub fn new_gateway(encoded_identity: String, node_id: NodeId) -> Self {
        TestableNode::new(encoded_identity, NodeType::Gateway, node_id)
    }

    pub fn is_mixnode(&self) -> bool {
        self.typ.is_mixnode()
    }
}

impl<'a> From<&'a mix::LegacyNode> for TestableNode {
    fn from(value: &'a mix::LegacyNode) -> Self {
        TestableNode {
            encoded_identity: value.identity_key.to_base58_string(),
            typ: NodeType::Mixnode,
            node_id: value.mix_id,
        }
    }
}

impl<'a> From<(&'a gateway::LegacyNode, NodeId)> for TestableNode {
    fn from((gateway, node_id): (&'a gateway::LegacyNode, NodeId)) -> Self {
        (&(gateway, node_id)).into()
    }
}

impl<'a> From<&'a (gateway::LegacyNode, NodeId)> for TestableNode {
    fn from((gateway, node_id): &'a (gateway::LegacyNode, NodeId)) -> Self {
        (gateway, *node_id).into()
    }
}

impl<'a, 'b> From<&'a (&'b gateway::LegacyNode, NodeId)> for TestableNode {
    fn from((gateway, node_id): &'a (&'b gateway::LegacyNode, NodeId)) -> Self {
        TestableNode {
            encoded_identity: gateway.identity_key.to_base58_string(),
            typ: NodeType::Gateway,
            node_id: *node_id,
        }
    }
}

impl Display for TestableNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-{}: {}",
            self.typ, self.node_id, self.encoded_identity
        )
    }
}

#[derive(Serialize, Deserialize, Hash, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Mixnode,
    Gateway,
}

impl NodeType {
    pub fn is_mixnode(&self) -> bool {
        matches!(self, NodeType::Mixnode { .. })
    }
}

impl Display for NodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Mixnode => write!(f, "mixnode"),
            NodeType::Gateway => write!(f, "gateway"),
        }
    }
}
