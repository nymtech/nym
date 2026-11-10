// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::PacketTrace;
use futures::channel::mpsc;
use futures::channel::mpsc::SendError;
use nym_sphinx::forwarding::packet::MixPacket;
use tokio::time::Instant;

pub fn mix_forwarding_channels() -> (MixForwardingSender, MixForwardingReceiver) {
    let (tx, rx) = mpsc::unbounded();
    (tx.into(), rx)
}

#[derive(Clone)]
pub struct MixForwardingSender(mpsc::UnboundedSender<PacketToForward>);

impl From<mpsc::UnboundedSender<PacketToForward>> for MixForwardingSender {
    fn from(tx: mpsc::UnboundedSender<PacketToForward>) -> Self {
        MixForwardingSender(tx)
    }
}

impl MixForwardingSender {
    pub fn forward_packet(&self, packet: PacketToForward) -> Result<(), SendError> {
        self.0
            .unbounded_send(packet)
            .map_err(|err| err.into_send_error())
    }

    // r[impl delay.gateways-no-delay]
    pub fn forward_client_packet_without_delay(&self, packet: MixPacket) -> Result<(), SendError> {
        self.forward_packet(PacketToForward::client_packet_without_delay(packet))
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub type MixForwardingReceiver = mpsc::UnboundedReceiver<PacketToForward>;

pub struct PacketToForward {
    pub packet: MixPacket,
    pub forward_delay_target: Option<Instant>,
    pub network_monitor_packet: bool,
    /// Latency breadcrumb started at packet receive; stamped as the packet moves through the
    /// forwarder and egress stages. `PacketTrace::Off` for untraced packets (e.g. acks).
    pub trace: PacketTrace,
}

impl PacketToForward {
    pub fn new(
        packet: MixPacket,
        forward_delay_target: Option<Instant>,
        network_monitor_packet: bool,
        trace: PacketTrace,
    ) -> Self {
        PacketToForward {
            packet,
            forward_delay_target,
            network_monitor_packet,
            trace,
        }
    }

    pub fn client_packet_without_delay(packet: MixPacket) -> Self {
        Self::new(packet, None, false, PacketTrace::Off)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use nym_sphinx::addressing::nodes::NymNodeRoutingAddress;
    use nym_sphinx::params::{PacketSize, PacketType, SphinxKeyRotation};
    use nym_sphinx::{
        Delay, Destination, DestinationAddressBytes, Node, NodeAddressBytes, NymPacket, PrivateKey,
        DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, NODE_ADDRESS_LENGTH,
    };

    fn mix_packet() -> MixPacket {
        let route = [Node::new(
            NodeAddressBytes::from_bytes([1u8; NODE_ADDRESS_LENGTH]),
            (&PrivateKey::random()).into(),
        )];
        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
            [4u8; IDENTIFIER_LENGTH],
        );
        let packet = NymPacket::sphinx_build_legacy(
            PacketSize::RegularPacket.payload_size(),
            b"x",
            &route,
            &destination,
            &[Delay::new_from_nanos(0)],
        )
        .expect("failed to build test sphinx packet");

        MixPacket::new(
            NymNodeRoutingAddress::Node("127.0.0.1:1789".parse().unwrap()),
            packet,
            PacketType::Mix,
            SphinxKeyRotation::Unknown,
        )
    }

    // r[verify delay.gateways-no-delay]
    #[test]
    fn a_client_packet_is_forwarded_with_no_delay() {
        let (sender, mut receiver) = mix_forwarding_channels();

        sender
            .forward_client_packet_without_delay(mix_packet())
            .unwrap();

        let forwarded = receiver.try_recv().expect("the packet must be forwarded");
        assert!(forwarded.forward_delay_target.is_none());
    }
}
