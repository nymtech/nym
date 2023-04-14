// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkTestingError;
use crate::MixId;
use nym_sphinx::message::NymMessage;
use nym_topology::{gateway, mix};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Serialize, Deserialize, Hash, Clone, Copy)]
pub enum NodeType {
    Mixnode(MixId),
    Gateway,
}

#[derive(Serialize, Deserialize, Hash, Clone, Copy)]
pub struct Empty;

#[derive(Serialize, Deserialize, Clone)]
pub struct TestMessage<T = Empty> {
    pub encoded_node_identity: String,
    pub node_owner: String,
    pub node_type: NodeType,

    pub msg_id: u32,
    pub total_msgs: u32,

    // any additional fields that might be required by a specific tester.
    // For example nym-api might want to attach route ids
    #[serde(flatten)]
    pub ext: T,
}

impl<T> TestMessage<T> {
    pub fn new_mix(node: &mix::Node, msg_id: u32, total_msgs: u32, ext: T) -> Self {
        TestMessage {
            encoded_node_identity: node.identity_key.to_base58_string(),
            node_owner: node.owner.clone(),
            node_type: NodeType::Mixnode(node.mix_id),
            msg_id,
            total_msgs,
            ext,
        }
    }

    pub fn new_gateway(node: &gateway::Node, msg_id: u32, total_msgs: u32, ext: T) -> Self {
        TestMessage {
            encoded_node_identity: node.identity_key.to_base58_string(),
            node_owner: node.owner.clone(),
            node_type: NodeType::Gateway,
            msg_id,
            total_msgs,
            ext,
        }
    }

    pub fn as_json_string(&self) -> Result<String, NetworkTestingError>
    where
        T: Serialize,
    {
        serde_json::to_string(self).map_err(Into::into)
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, NetworkTestingError>
    where
        T: Serialize,
    {
        // the test messages are supposed to be rather small so we can use the good old serde_json
        // (the performance penalty over bincode or custom serialization should be minimal)
        serde_json::to_vec(self).map_err(Into::into)
    }

    pub fn try_recover(msg: NymMessage) -> Result<Self, NetworkTestingError>
    where
        T: DeserializeOwned,
    {
        let inner = msg.into_inner_data();
        Self::try_recover_from_bytes(&inner)
    }

    pub fn try_recover_from_bytes(raw: &[u8]) -> Result<Self, NetworkTestingError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice(raw)
            .map_err(|source| NetworkTestingError::MalformedTestMessageReceived { source })
    }
}

impl<T: Hash> Hash for TestMessage<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.encoded_node_identity.hash(state);
        self.node_owner.hash(state);
        self.node_type.hash(state);
        self.ext.hash(state)
    }
}
