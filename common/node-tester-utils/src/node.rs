// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::MixId;
use nym_topology::{gateway, mix};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct TestableNode {
    pub encoded_identity: String,
    pub owner: String,

    #[serde(rename = "type")]
    pub typ: NodeType,
}

impl TestableNode {
    pub fn new(encoded_identity: String, owner: String, typ: NodeType) -> Self {
        TestableNode {
            encoded_identity,
            owner,
            typ,
        }
    }

    pub fn new_mixnode(encoded_identity: String, owner: String, mix_id: MixId) -> Self {
        TestableNode::new(encoded_identity, owner, NodeType::Mixnode { mix_id })
    }

    pub fn new_gateway(encoded_identity: String, owner: String) -> Self {
        TestableNode::new(encoded_identity, owner, NodeType::Gateway)
    }

    pub fn is_mixnode(&self) -> bool {
        self.typ.is_mixnode()
    }
}

impl<'a> From<&'a mix::Node> for TestableNode {
    fn from(value: &'a mix::Node) -> Self {
        TestableNode {
            encoded_identity: value.identity_key.to_base58_string(),
            owner: value.owner.as_ref().cloned().unwrap_or_default(),
            typ: NodeType::Mixnode {
                mix_id: value.mix_id,
            },
        }
    }
}

impl<'a> From<&'a gateway::Node> for TestableNode {
    fn from(value: &'a gateway::Node) -> Self {
        TestableNode {
            encoded_identity: value.identity_key.to_base58_string(),
            owner: value.owner.as_ref().cloned().unwrap_or_default(),
            typ: NodeType::Gateway,
        }
    }
}

impl Display for TestableNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} owned by {}",
            self.typ, self.encoded_identity, self.owner
        )
    }
}

#[derive(Serialize, Deserialize, Hash, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Mixnode { mix_id: MixId },
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
            NodeType::Mixnode { mix_id } => write!(f, "mixnode (mix_id {mix_id})"),
            NodeType::Gateway => write!(f, "gateway"),
        }
    }
}
