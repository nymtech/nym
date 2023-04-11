// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::message::NymMessage;
use nym_sphinx::params::{PacketSize, DEFAULT_NUM_MIX_HOPS};
use nym_sphinx::preparer::{FragmentPreparer, PreparedFragment};
use nym_topology::{gateway, mix, MixLayer, NymTopology, NymTopologyError};
use rand::{CryptoRng, Rng, RngCore};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

// it feels wrong to redefine it, but I don't want to import the whole of contract commons just for this one type
type MixId = u32;

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

    // any additional fields that might be required by a specific tester.
    // For example nym-api might want to attach route ids
    #[serde(flatten)]
    pub ext: T,
}

impl<T> TestMessage<T> {
    pub fn new_mix(node: &mix::Node, ext: T) -> Self {
        TestMessage {
            encoded_node_identity: node.identity_key.to_base58_string(),
            node_owner: node.owner.clone(),
            node_type: NodeType::Mixnode(node.mix_id),
            ext,
        }
    }

    pub fn new_gateway(node: &gateway::Node, ext: T) -> Self {
        TestMessage {
            encoded_node_identity: node.identity_key.to_base58_string(),
            node_owner: node.owner.clone(),
            node_type: NodeType::Gateway,
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
}

impl<T: Hash> Hash for TestMessage<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.encoded_node_identity.hash(state);
        self.node_owner.hash(state);
        self.node_type.hash(state);
        self.ext.hash(state)
    }
}

#[derive(Debug, Error)]
pub enum NetworkTestingError {
    #[error(transparent)]
    SerializationFailure(#[from] serde_json::Error),

    #[error(transparent)]
    InvalidTopology(#[from] NymTopologyError),

    #[error("The specified mixnode (id: {mix_id}) doesn't exist")]
    NonExistentMixnode { mix_id: MixId },

    #[error("The specified gateway (id: {gateway_identity}) doesn't exist")]
    NonExistentGateway { gateway_identity: String },

    #[error("The provided test message is too long to fit in a single sphinx packet")]
    TestMessageTooLong,
}

pub struct NodeTester<R> {
    rng: R,

    base_topology: NymTopology,

    recipient: Recipient,

    packet_size: PacketSize,

    /// Average delay a data packet is going to get delay at a single mixnode.
    average_packet_delay: Duration,

    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: Duration,

    /// Number of mix hops each packet ('real' message, ack, reply) is expected to take.
    /// Note that it does not include gateway hops.
    num_mix_hops: u8,

    // while acks are going to be ignored they still need to be constructed
    // so that the gateway would be able to correctly process and forward the message
    ack_key: Arc<AckKey>,
}

// TODO: to use it in the following PR
#[allow(dead_code)]
impl<R> NodeTester<R>
where
    R: Rng + CryptoRng,
{
    pub fn new(
        rng: R,
        base_topology: NymTopology,
        recipient: Recipient,
        packet_size: PacketSize,
        average_packet_delay: Duration,
        average_ack_delay: Duration,
        ack_key: Arc<AckKey>,
    ) -> Self {
        Self {
            rng,
            base_topology,
            recipient,
            packet_size,
            average_packet_delay,
            average_ack_delay,
            num_mix_hops: DEFAULT_NUM_MIX_HOPS,
            ack_key,
        }
    }

    /// Allows setting non-default number of expected mix hops in the network.
    #[allow(dead_code)]
    pub fn with_mix_hops(mut self, hops: u8) -> Self {
        self.num_mix_hops = hops;
        self
    }

    fn testable_mix_topology(&self, node: &mix::Node) -> NymTopology {
        let mut topology = self.base_topology.clone();
        topology.set_mixes_in_layer(node.layer as u8, vec![node.clone()]);
        topology
    }

    fn testable_gateway_topology(&self, gateway: &gateway::Node) -> NymTopology {
        let mut topology = self.base_topology.clone();
        topology.set_gateways(vec![gateway.clone()]);
        topology
    }

    fn test_mixnode_simple(&mut self, mix: &mix::Node) -> Result<(), NetworkTestingError> {
        self.test_mixnode(mix, Empty)
    }

    fn test_mixnode<T>(&mut self, mix: &mix::Node, msg_ext: T) -> Result<(), NetworkTestingError>
    where
        T: Serialize,
    {
        let msg = TestMessage::new_mix(mix, msg_ext);
        let ephemeral_topology = self.testable_mix_topology(mix);
        self.create_test_packet(msg, &ephemeral_topology)?;

        todo!()
    }

    fn test_existing_mixnode<T>(
        &mut self,
        mix_id: MixId,
        msg_ext: T,
    ) -> Result<(), NetworkTestingError>
    where
        T: Serialize,
    {
        let Some(node) = self.base_topology.find_mix(mix_id) else {
            return Err(NetworkTestingError::NonExistentMixnode {mix_id})
        };

        self.test_mixnode(&node.clone(), msg_ext)
    }

    fn create_test_packet<T>(
        &mut self,
        message: TestMessage<T>,
        topology: &NymTopology,
    ) -> Result<PreparedFragment, NetworkTestingError>
    where
        T: Serialize,
    {
        let serialized = message.as_bytes()?;
        let message = NymMessage::new_plain(serialized);

        let mut fragments = self.pad_and_split_message(message, self.packet_size);

        if fragments.len() != 1 {
            return Err(NetworkTestingError::TestMessageTooLong);
        }

        // SAFETY: the unwrap here is fine as if the vec was somehow empty
        // we would have returned the error when checking for its length
        let fragment = fragments.pop().unwrap();

        // the packet is designed to be sent from ourselves to ourselves
        let address = self.recipient;

        // TODO: can we avoid this arc clone?
        let ack_key = Arc::clone(&self.ack_key);
        Ok(self.prepare_chunk_for_sending(fragment, topology, &ack_key, &address, &address)?)
    }
}

impl<R: CryptoRng + Rng> FragmentPreparer for NodeTester<R> {
    type Rng = R;

    fn rng(&mut self) -> &mut Self::Rng {
        &mut self.rng
    }

    fn num_mix_hops(&self) -> u8 {
        self.num_mix_hops
    }

    fn average_packet_delay(&self) -> Duration {
        self.average_packet_delay
    }

    fn average_ack_delay(&self) -> Duration {
        self.average_ack_delay
    }
}
