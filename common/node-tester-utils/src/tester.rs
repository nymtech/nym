// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkTestingError;
use crate::Empty;
use crate::NodeId;
use crate::TestMessage;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::message::NymMessage;
use nym_sphinx::params::{PacketSize, DEFAULT_NUM_MIX_HOPS};
use nym_sphinx::preparer::{FragmentPreparer, PreparedFragment};
use nym_sphinx_params::PacketType;
use nym_topology::{gateway, mix, NymTopology};
use rand::{CryptoRng, Rng};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

pub struct NodeTester<R> {
    rng: R,

    base_topology: NymTopology,

    /// Generally test packets are designed to be sent from ourselves to ourselves,
    /// However, one might want to customise this behaviour.
    /// In that case an explicit `Recipient` has to be provided when constructing test packets.
    self_address: Option<Recipient>,

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

impl<R> NodeTester<R>
where
    R: Rng + CryptoRng,
{
    pub fn new(
        rng: R,
        base_topology: NymTopology,
        self_address: Option<Recipient>,
        packet_size: PacketSize,
        average_packet_delay: Duration,
        average_ack_delay: Duration,
        ack_key: Arc<AckKey>,
    ) -> Self {
        Self {
            rng,
            base_topology,
            self_address,
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

    pub fn testable_mix_topology(&self, node: &mix::LegacyNode) -> NymTopology {
        let mut topology = self.base_topology.clone();
        topology.set_mixes_in_layer(node.layer as u8, vec![node.clone()]);
        topology
    }

    pub fn testable_gateway_topology(&self, gateway: &gateway::LegacyNode) -> NymTopology {
        let mut topology = self.base_topology.clone();
        topology.set_gateways(vec![gateway.clone()]);
        topology
    }

    pub fn simple_mixnode_test_packets(
        &mut self,
        mix: &mix::LegacyNode,
        test_packets: u32,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError> {
        self.mixnode_test_packets(mix, Empty, test_packets, None)
    }

    pub fn mixnode_test_packets<T>(
        &mut self,
        mix: &mix::LegacyNode,
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let ephemeral_topology = self.testable_mix_topology(mix);

        let mut packets = Vec::with_capacity(test_packets as usize);
        for plaintext in TestMessage::mix_plaintexts(mix, test_packets, msg_ext)? {
            packets.push(self.wrap_plaintext_data(
                plaintext,
                &ephemeral_topology,
                custom_recipient,
            )?);
        }

        Ok(packets)
    }

    pub fn mixnodes_test_packets<T>(
        &mut self,
        nodes: &[mix::LegacyNode],
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let mut packets = Vec::new();
        for node in nodes {
            packets.append(&mut self.mixnode_test_packets(
                node,
                msg_ext.clone(),
                test_packets,
                custom_recipient,
            )?)
        }

        Ok(packets)
    }

    pub fn existing_mixnode_test_packets<T>(
        &mut self,
        mix_id: NodeId,
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let Some(node) = self.base_topology.find_mix(mix_id) else {
            return Err(NetworkTestingError::NonExistentMixnode { mix_id });
        };

        self.mixnode_test_packets(&node.clone(), msg_ext, test_packets, custom_recipient)
    }

    pub fn existing_identity_mixnode_test_packets<T>(
        &mut self,
        encoded_mix_identity: String,
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let Some(node) = self
            .base_topology
            .find_mix_by_identity(&encoded_mix_identity)
        else {
            return Err(NetworkTestingError::NonExistentMixnodeIdentity {
                mix_identity: encoded_mix_identity,
            });
        };

        self.mixnode_test_packets(&node.clone(), msg_ext, test_packets, custom_recipient)
    }

    pub fn legacy_gateway_test_packets<T>(
        &mut self,
        gateway: &gateway::LegacyNode,
        node_id: NodeId,
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let ephemeral_topology = self.testable_gateway_topology(gateway);

        let mut packets = Vec::with_capacity(test_packets as usize);
        for plaintext in
            TestMessage::legacy_gateway_plaintexts(gateway, node_id, test_packets, msg_ext)?
        {
            packets.push(self.wrap_plaintext_data(
                plaintext,
                &ephemeral_topology,
                custom_recipient,
            )?);
        }

        Ok(packets)
    }

    pub fn existing_gateway_test_packets<T>(
        &mut self,
        node_id: NodeId,
        encoded_gateway_identity: String,
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let Some(node) = self.base_topology.find_gateway(&encoded_gateway_identity) else {
            return Err(NetworkTestingError::NonExistentGateway {
                gateway_identity: encoded_gateway_identity,
            });
        };

        self.legacy_gateway_test_packets(
            &node.clone(),
            node_id,
            msg_ext,
            test_packets,
            custom_recipient,
        )
    }

    pub fn wrap_plaintext_data(
        &mut self,
        plaintext: Vec<u8>,
        topology: &NymTopology,
        custom_recipient: Option<Recipient>,
    ) -> Result<PreparedFragment, NetworkTestingError> {
        let message = NymMessage::new_plain(plaintext);

        let mut fragments = self.pad_and_split_message(message, self.packet_size);

        if fragments.len() != 1 {
            return Err(NetworkTestingError::TestMessageTooLong);
        }

        // SAFETY: the unwrap here is fine as if the vec was somehow empty
        // we would have returned the error when checking for its length
        let fragment = fragments.pop().unwrap();

        // either `self_address` or `custom_recipient` has to be specified.
        let address = custom_recipient.unwrap_or(
            self.self_address
                .ok_or(NetworkTestingError::UnknownPacketRecipient)?,
        );

        // TODO: can we avoid this arc clone?
        let ack_key = Arc::clone(&self.ack_key);
        Ok(self.prepare_chunk_for_sending(
            fragment,
            topology,
            &ack_key,
            &address,
            &address,
            PacketType::Mix,
            None,
        )?)
    }

    pub fn create_test_packet<T>(
        &mut self,
        message: &TestMessage<T>,
        topology: &NymTopology,
        custom_recipient: Option<Recipient>,
    ) -> Result<PreparedFragment, NetworkTestingError>
    where
        T: Serialize,
    {
        let serialized = message.as_bytes()?;
        self.wrap_plaintext_data(serialized, topology, custom_recipient)
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

    fn nonce(&self) -> i32 {
        1
    }
}
