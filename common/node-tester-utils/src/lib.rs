// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::message::NymMessage;
use nym_sphinx::params::PacketSize;
use nym_topology::{gateway, mix, NymTopology};
use rand::{Rng, RngCore};
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlaceholderError {
    #[error("TEMP")]
    SerdeJsonPlaceholder(#[from] serde_json::Error),
}

// no need for strong(crypto) rng here
pub struct NodeTester<R> {
    rng: R,

    base_topology: NymTopology,
    recipient: Recipient,

    packet_size: PacketSize,
    num_mix_hops: u8,

    // while acks are going to be ignored they still need to be constructed
    // so that the gateway would be able to correctly processed the message
    ack_key: Arc<AckKey>,
}

impl<R> NodeTester<R> {
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

    fn test_mixnode(&self, mix_id: u32) -> Result<(), PlaceholderError> {
        let Some(node) = self.base_topology.find_mix(mix_id) else {
            todo!()
        };

        todo!()
    }

    fn temp<T>(&mut self, data: &T) -> Result<MixPacket, PlaceholderError>
    where
        T: Serialize,
        R: Rng + RngCore,
    {
        // the test messages are supposed to be rather small so we can use the good old serde_json
        // (the performance penalty over bincode or custom serialization should be minimal)
        let serialized = serde_json::to_vec(data)?;
        let message = NymMessage::new_plain(serialized);

        let plaintext_per_packet = message.available_plaintext_per_packet(self.packet_size);

        let mut fragments = message
            .pad_to_full_packet_lengths(plaintext_per_packet)
            .split_into_fragments(&mut self.rng, plaintext_per_packet);

        if fragments.len() != 1 {
            panic!("todo")
        }

        // SAFETY: the unwrap here is fine as if the vec was somehow empty
        // we would have returned the error when checking for its length
        let fragment = fragments.pop().unwrap();

        todo!()
    }

    fn as_raw(&self) {
        todo!()
    }

    /// Intended to be used inside a pre-existing processing pipeline.
    /// It's supposed to 'trick' the receiver to think its a properly fragmented message.
    fn as_fragmented(&self) {
        //
    }
}

// TestMessage: Serialize
