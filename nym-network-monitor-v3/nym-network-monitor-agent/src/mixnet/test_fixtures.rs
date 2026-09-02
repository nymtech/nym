// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Fixtures shared by the mixnet module's tests.

use crate::agent::tested_node::TestedNodeDetails;
use crate::mixnet::events::{IngressEvent, IngressEventsReceiver};
use crate::mixnet::sphinx::helpers::{as_sphinx_node, create_test_sphinx_packet_header};
use crate::mixnet::sphinx::payload::PayloadRecovery;
use crate::mixnet::sphinx::test_packet::{TestPacketContent, TestPacketHeader};
use crate::mixnet::targets::WaveTarget;
use futures::channel::mpsc::unbounded;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_params::PacketType;
use nym_sphinx_types::{DESTINATION_ADDRESS_LENGTH, DestinationAddressBytes, NymPacket};
use rand::rngs::OsRng;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

pub(crate) fn ip(raw: &str) -> IpAddr {
    raw.parse().expect("malformed test ip")
}

pub(crate) fn socket(raw: &str) -> SocketAddr {
    raw.parse().expect("malformed test socket address")
}

/// One target of a wave from the outside: what the ingress is built from, the receiving end of its
/// channel, and the header its returned packets are built with.
///
/// The header is the load-bearing part. Packets are created and recovered through the SAME reusable
/// header a real probe of this target would use, so a packet routed to the wrong target cannot be
/// decrypted there. Attribution failures therefore surface as a decryption error rather than as a
/// count that happens to match.
pub(crate) struct ProbedTarget {
    target: WaveTarget,
    received: IngressEventsReceiver,
    header: TestPacketHeader,
}

impl ProbedTarget {
    pub(crate) fn new(address: SocketAddr, known_ips: &[IpAddr]) -> Self {
        let node = TestedNodeDetails::new_test(address, known_ips);

        // the agent hop only has to be a well-formed final hop; nothing here processes the packet
        // through a node, so the keys need not correspond to anything real
        let agent_key = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut OsRng));
        let route = [
            node.as_sphinx_node(),
            as_sphinx_node(socket("127.0.0.1:9000"), agent_key),
        ];
        // nothing in these tests resolves a destination: the agent hop unwraps the payload itself, so
        // any well-formed address will do
        let client_address = DestinationAddressBytes::from_bytes([7u8; DESTINATION_ADDRESS_LENGTH]);
        let header =
            create_test_sphinx_packet_header(&route, client_address, Duration::from_millis(50))
                .expect("failed to build the fixture's reusable header");

        let (events, received) = unbounded();
        ProbedTarget {
            target: WaveTarget { node, events },
            received,
            header,
        }
    }

    /// What the wave's ingress is assembled from. Cloned rather than moved out, so a test can keep
    /// asserting on this fixture after handing its target over.
    pub(crate) fn wave_target(&self) -> WaveTarget {
        WaveTarget {
            node: self.target.node.clone(),
            events: self.target.events.clone(),
        }
    }

    pub(crate) fn noise_key(&self) -> x25519::PublicKey {
        self.target.node.noise_key
    }

    /// How a probe of this target would decrypt what it returns, i.e. through the same reusable
    /// header [`reply`](Self::reply) builds with.
    pub(crate) fn payload_recovery(&self) -> PayloadRecovery {
        self.header.clone().into()
    }

    /// A packet this target would return, carrying `id`, in the form the agent sees it: the node has
    /// already peeled its own payload layer by the time it forwards.
    pub(crate) fn reply(&self, id: u64) -> FramedNymPacket {
        let packet = self
            .header
            .create_returned_packet(TestPacketContent::new(id))
            .expect("failed to build a fixture reply");

        FramedNymPacket::new(
            NymPacket::Sphinx(packet),
            PacketType::Mix,
            self.target.node.key_rotation,
            false,
        )
    }

    /// Every event currently queued for this target.
    pub(crate) fn drain(&mut self) -> Vec<IngressEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.received.try_recv() {
            events.push(event);
        }
        events
    }

    /// The ids of the packets routed to this target, recovered with its OWN payload key. Non-packet
    /// events are skipped; use [`drain`](Self::drain) to assert on those.
    pub(crate) fn received_ids(&mut self) -> Vec<u64> {
        let events = self.drain();

        events
            .into_iter()
            .filter_map(|event| match event {
                IngressEvent::Packet(received) => {
                    let sphinx = received
                        .received
                        .into_inner()
                        .to_sphinx_packet()
                        .expect("a routed packet was not a sphinx packet");
                    let content = self.header.recover_payload(sphinx.payload).expect(
                        "a packet routed to this target could not be decrypted with its own key",
                    );
                    Some(content.id)
                }
                _ => None,
            })
            .collect()
    }
}
