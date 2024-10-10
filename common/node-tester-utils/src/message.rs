// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkTestingError;
use crate::node::TestableNode;
use crate::NodeId;
use nym_sphinx::message::NymMessage;
use nym_topology::{gateway, mix};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Empty;

#[derive(Serialize, Deserialize, Clone)]
pub struct TestMessage<T = Empty> {
    pub tested_node: TestableNode,

    pub msg_id: u32,
    pub total_msgs: u32,

    // any additional fields that might be required by a specific tester.
    // For example nym-api might want to attach route ids
    #[serde(flatten)]
    pub ext: T,
}

impl<T> TestMessage<T> {
    pub fn new<N: Into<TestableNode>>(node: N, msg_id: u32, total_msgs: u32, ext: T) -> Self {
        TestMessage {
            tested_node: node.into(),
            msg_id,
            total_msgs,
            ext,
        }
    }

    pub fn new_mix(node: &mix::LegacyNode, msg_id: u32, total_msgs: u32, ext: T) -> Self {
        Self::new(node, msg_id, total_msgs, ext)
    }

    // pub fn new_gateway(node: &gateway::Node, msg_id: u32, total_msgs: u32, ext: T) -> Self {
    //     Self::new(node, msg_id, total_msgs, ext)
    // }

    pub fn new_serialized<N>(
        node: N,
        msg_id: u32,
        total_msgs: u32,
        ext: T,
    ) -> Result<Vec<u8>, NetworkTestingError>
    where
        N: Into<TestableNode>,
        T: Serialize,
    {
        Self::new(node, msg_id, total_msgs, ext).as_bytes()
    }

    pub fn new_plaintexts<N>(
        node: &N,
        total_msgs: u32,
        ext: T,
    ) -> Result<Vec<Vec<u8>>, NetworkTestingError>
    where
        for<'a> &'a N: Into<TestableNode>,
        T: Serialize + Clone,
    {
        let mut msgs = Vec::with_capacity(total_msgs as usize);
        for msg_id in 1..=total_msgs {
            msgs.push(Self::new(node, msg_id, total_msgs, ext.clone()).as_bytes()?)
        }
        Ok(msgs)
    }

    pub fn mix_plaintexts(
        node: &mix::LegacyNode,
        total_msgs: u32,
        ext: T,
    ) -> Result<Vec<Vec<u8>>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        Self::new_plaintexts(node, total_msgs, ext)
    }

    pub fn legacy_gateway_plaintexts(
        node: &gateway::LegacyNode,
        node_id: NodeId,
        total_msgs: u32,
        ext: T,
    ) -> Result<Vec<Vec<u8>>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        Self::new_plaintexts(&(node, node_id), total_msgs, ext)
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
