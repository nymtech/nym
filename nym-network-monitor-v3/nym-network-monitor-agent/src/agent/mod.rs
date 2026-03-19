// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::test_packet::TestPacketContent;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_params::PacketSize;
use nym_sphinx_types::{
    DESTINATION_ADDRESS_LENGTH, Delay, Destination, DestinationAddressBytes, IDENTIFIER_LENGTH,
    NymPacket,
};
use std::net::SocketAddr;

pub(crate) mod listener;
pub(crate) mod receiver;
pub(crate) mod test_packet;

/// Configuration for the [`NetworkMonitorAgent`], controlling packet sending behaviour during a test run.
pub(crate) struct Config {
    /// How long the agent should be sending test packets with the specified rate.
    pub(crate) sending_duration: std::time::Duration,

    /// How long the agent will wait to receive any leftover packets after finishing sending.
    pub(crate) waiting_duration: std::time::Duration,

    /// How long the node itself should delay the packet
    pub(crate) packet_delay: std::time::Duration,

    /// Target rate of packets (per second) to be sent.
    pub(crate) target_rate: usize,

    /// Whether the agent should reuse the same header for all packets, and consequently replay them.
    pub(crate) reuse_header: bool,

    /// Address of the mixnet listener on this agent
    pub(crate) mixnet_address: SocketAddr,
}

pub(crate) struct TestedNodeDetails {
    pub(crate) address: SocketAddr,

    pub(crate) noise_key: x25519::PublicKey,

    pub(crate) sphinx_key: x25519::PublicKey,
}

pub(crate) struct NetworkMonitorAgent {
    config: Config,

    noise_key: x25519::PrivateKey,

    sphinx_key: x25519::PrivateKey,

    tested_node: TestedNodeDetails,
}

pub(crate) struct TestRunResult {
    //
}

impl NetworkMonitorAgent {
    fn as_sphinx_node(&self) -> anyhow::Result<nym_sphinx_types::Node> {
        Ok(nym_sphinx_types::Node::new(
            NymNodeRoutingAddress::from(self.config.mixnet_address).try_into()?,
            self.sphinx_key.public_key().into(),
        ))
    }

    // if needed, we could use it for additional data
    fn dummy_destination(&self) -> Destination {
        Destination::new(
            DestinationAddressBytes::from_bytes([0u8; DESTINATION_ADDRESS_LENGTH]),
            [0u8; IDENTIFIER_LENGTH],
        )
    }

    fn create_test_sphinx_packet(&self, content: TestPacketContent) -> anyhow::Result<NymPacket> {
        // we don't want any delays
        // and the packet route is test node -> this client
        let route = [self.as_sphinx_node()?];
        let delays = [Delay::new_from_nanos(
            self.config.packet_delay.as_nanos() as u64
        )];
        let destination = self.dummy_destination();
        // we use acks for their reduced size
        let payload = PacketSize::AckPacket.payload_size();

        let forward_packet = NymPacket::sphinx_build(
            false,
            payload,
            content.to_bytes(),
            &route,
            &destination,
            &delays,
        )?;
        Ok(forward_packet)
    }

    pub(crate) async fn run_stress_test(&self) -> anyhow::Result<TestRunResult> {
        // 1. send a single packet to see if the node is even going to respond to it

        // 2. send it again to check if the node is configured correctly for testing
        // (i.e. whether the agent can bypass the bloomfilter)

        // 3. finally, send the packets at the pre-defined rate to see if it can handle the target load
        todo!()
    }
}
