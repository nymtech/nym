// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkTestingError;
use crate::TestMessage;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::message::NymMessage;
use nym_sphinx::params::PacketSize;
use nym_sphinx::preparer::{FragmentPreparer, PreparedFragment};
use nym_sphinx_params::PacketType;
use nym_topology::node::LegacyMixLayer;
use nym_topology::node::RoutingNode;
use nym_topology::{NymRouteProvider, NymTopology, Role};
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

    /// Specify whether route selection should be determined by the packet header.
    deterministic_route_selection: bool,

    /// Average delay a data packet is going to get delay at a single mixnode.
    average_packet_delay: Duration,

    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: Duration,

    // while acks are going to be ignored they still need to be constructed
    // so that the gateway would be able to correctly process and forward the message
    ack_key: Arc<AckKey>,
}

impl<R> NodeTester<R>
where
    R: Rng + CryptoRng,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rng: R,
        base_topology: NymTopology,
        self_address: Option<Recipient>,
        packet_size: PacketSize,
        deterministic_route_selection: bool,
        average_packet_delay: Duration,
        average_ack_delay: Duration,
        ack_key: Arc<AckKey>,
    ) -> Self {
        Self {
            rng,
            base_topology,
            self_address,
            packet_size,
            deterministic_route_selection,
            average_packet_delay,
            average_ack_delay,
            ack_key,
        }
    }

    pub fn testable_mix_topology(&self, layer: LegacyMixLayer, node: &RoutingNode) -> NymTopology {
        let mut topology = self.base_topology.clone();
        topology.set_testable_node(layer.into(), node.clone());
        topology
    }

    pub fn testable_gateway_topology(&self, node: &RoutingNode) -> NymTopology {
        let mut topology = self.base_topology.clone();
        topology.set_testable_node(Role::EntryGateway, node.clone());
        topology.set_testable_node(Role::ExitGateway, node.clone());
        topology
    }

    pub fn mixnode_test_packets<T>(
        &mut self,
        mix: &RoutingNode,
        legacy_mix_layer: LegacyMixLayer,
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let ephemeral_topology =
            NymRouteProvider::from(self.testable_mix_topology(legacy_mix_layer, mix))
                .with_ignore_egress_epoch_roles(true);

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
        nodes: &[(LegacyMixLayer, RoutingNode)],
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let mut packets = Vec::new();
        for (layer, node) in nodes {
            packets.append(&mut self.mixnode_test_packets(
                node,
                *layer,
                msg_ext.clone(),
                test_packets,
                custom_recipient,
            )?)
        }

        Ok(packets)
    }

    pub fn existing_identity_mixnode_test_packets<T>(
        &mut self,
        encoded_mix_identity: String,
        layer: LegacyMixLayer,
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let Ok(identity) = encoded_mix_identity.parse() else {
            return Err(NetworkTestingError::NonExistentMixnodeIdentity {
                mix_identity: encoded_mix_identity,
            });
        };

        let Some(node) = self.base_topology.find_node_by_identity(identity) else {
            return Err(NetworkTestingError::NonExistentMixnodeIdentity {
                mix_identity: encoded_mix_identity,
            });
        };

        self.mixnode_test_packets(
            &node.clone(),
            layer,
            msg_ext,
            test_packets,
            custom_recipient,
        )
    }

    pub fn legacy_gateway_test_packets<T>(
        &mut self,
        gateway: &RoutingNode,
        msg_ext: T,
        test_packets: u32,
        custom_recipient: Option<Recipient>,
    ) -> Result<Vec<PreparedFragment>, NetworkTestingError>
    where
        T: Serialize + Clone,
    {
        let ephemeral_topology = NymRouteProvider::from(self.testable_gateway_topology(gateway))
            .with_ignore_egress_epoch_roles(true);

        let mut packets = Vec::with_capacity(test_packets as usize);
        for plaintext in TestMessage::legacy_gateway_plaintexts(gateway, test_packets, msg_ext)? {
            packets.push(self.wrap_plaintext_data(
                plaintext,
                &ephemeral_topology,
                custom_recipient,
            )?);
        }

        Ok(packets)
    }

    pub fn wrap_plaintext_data(
        &mut self,
        plaintext: Vec<u8>,
        topology: &NymRouteProvider,
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
        )?)
    }

    pub fn create_test_packet<T>(
        &mut self,
        message: &TestMessage<T>,
        topology: &NymRouteProvider,
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

    fn deterministic_route_selection(&self) -> bool {
        self.deterministic_route_selection
    }

    fn rng(&mut self) -> &mut Self::Rng {
        &mut self.rng
    }

    fn nonce(&self) -> i32 {
        1
    }

    fn average_packet_delay(&self) -> Duration {
        self.average_packet_delay
    }

    fn average_ack_delay(&self) -> Duration {
        self.average_ack_delay
    }
}
