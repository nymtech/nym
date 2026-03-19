// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::sphinx_helpers::{
    TestPacketHeader, create_test_sphinx_packet_header, rederive_expanded_shared_secret,
};
use crate::agent::test_packet::TestPacketContent;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_params::PacketSize;
use nym_sphinx_types::{
    DESTINATION_ADDRESS_LENGTH, Delay, Destination, DestinationAddressBytes, IDENTIFIER_LENGTH,
    Payload, PayloadKey, SphinxHeader, SphinxPacket, SphinxPacketBuilder,
};
use rand::rngs::OsRng;
use std::net::SocketAddr;
use x25519_dalek::{PublicKey, StaticSecret};

pub(crate) mod listener;
pub(crate) mod receiver;
mod sphinx_helpers;
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

impl TestedNodeDetails {
    pub(crate) fn as_sphinx_node(&self) -> anyhow::Result<nym_sphinx_types::Node> {
        Ok(nym_sphinx_types::Node::new(
            NymNodeRoutingAddress::from(self.address).try_into()?,
            self.sphinx_key.into(),
        ))
    }
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

    fn create_test_sphinx_packet_header(&self) -> anyhow::Result<TestPacketHeader> {
        // we don't want any delays
        // and the packet route is test node -> this client
        let route = vec![self.tested_node.as_sphinx_node()?, self.as_sphinx_node()?];
        let delay = self.config.packet_delay;
        create_test_sphinx_packet_header(route, delay)
    }

    // fn create_test_header(&self) -> anyhow::Result<TestPacketHeader> {
    //     let dummy_packet = self
    //         .create_test_sphinx_packet()?
    //         .to_sphinx_packet()
    //         .unwrap();
    //     let header = dummy_packet.header;
    //
    //     let shared_secret = sphinx_key.inner().diffie_hellman(&header.shared_secret);
    //     let payload_key = rederive_expanded_shared_secret(shared_secret.as_bytes());
    //
    //     let header = SphinxHeader::new();
    //     let payload_key = PayloadKey::new();
    //     TestPacketHeader {
    //         header,
    //         payload_key,
    //     }
    // }
    //
    // pub(crate) async fn run_stress_test(&self) -> anyhow::Result<TestRunResult> {
    //     let test_packet = self
    //         .create_test_sphinx_packet()?
    //         .to_sphinx_packet()
    //         .unwrap();
    //     // let test_header = test_packet.header;
    //     // let payload_keys = test_header.payload_keys();
    //
    //     // 1. send a single packet to see if the node is even going to respond to it
    //
    //     // 2. send it again to check if the node is configured correctly for testing
    //     // (i.e. whether the agent can bypass the bloomfilter)
    //
    //     // 3. finally, send the packets at the pre-defined rate to see if it can handle the target load
    //     todo!()
    // }
}

// we have to recreate the payload key creation and manually set the content
fn set_packet_payload(
    header: SphinxHeader,
    new_payload: &[u8],
    sphinx_key: &x25519::PrivateKey,
) -> anyhow::Result<SphinxPacket> {
    // 1. rederive expanded secrets

    // derive the expanded shared secret for our node so we could tag the payload to figure out latency
    // by tagging the packet
    let shared_secret = sphinx_key.inner().diffie_hellman(&header.shared_secret);
    let payload_key = rederive_expanded_shared_secret(shared_secret.as_bytes());
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swapping_sphinx_packet_content() {
        //
    }
}
