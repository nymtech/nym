// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_common::debug::format_debug_bytes;
use nym_sphinx::{Delay, SphinxPacketBuilder};

use rand::Rng;

use std::fmt::Debug;

use crate::{
    helpers,
    node::NodeId,
    packet::WirePacketFormat,
    topology::directory::{Directory, DirectoryClient},
};

/// On-wire packet exchanged between mix nodes in the Sphinx pipeline.
///
/// Wraps a serialised Sphinx packet as a `Vec<u8>` and supplies a
/// [`WirePacketFormat`] impl plus a trimmed [`Debug`] implementation that shows
/// only the first 32 bytes of the serialised form to avoid flooding logs.
pub struct SimMixPacket(Vec<u8>);

impl Debug for SimMixPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SimMixPacket {{")?;
        writeln!(f, "    data start:")?;
        if self.0.len() > 32 {
            for line in format_debug_bytes(&self.0.to_bytes()[..32])?.lines() {
                writeln!(f, "        {line}")?;
            }
        } else {
            for line in format_debug_bytes(&self.0.to_bytes())?.lines() {
                writeln!(f, "        {line}")?;
            }
        }
        write!(f, "}}")
    }
}

impl WirePacketFormat for SimMixPacket {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(SimMixPacket(bytes.to_vec()))
    }
}

impl From<Vec<u8>> for SimMixPacket {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<SimMixPacket> for Vec<u8> {
    fn from(value: SimMixPacket) -> Self {
        value.0
    }
}

/// A pre-built Sphinx packet that the recipient sends back as an acknowledgement.
///
/// A `SurbAck` bundles the serialised Sphinx packet together with the first-hop
/// node ID and the expected total mix delay so that the sender can compute the
/// latest time by which the ACK should arrive.
#[derive(Debug)]
pub struct SurbAck {
    surb_ack_packet: SimMixPacket,
    first_hop_id: NodeId,
    expected_total_delay: Delay,
}

impl SurbAck {
    /// Magic bytes written at the start of every SURB ACK payload so that the
    /// final-hop node can identify them and route them separately.
    pub const MARKER: &[u8; 8] = b"SURB_ACK";
    const ACK_SIZE: usize = 8 + 8; // u64 ID and MARKER
    const PAYLOAD_SIZE: usize = Self::ACK_SIZE + nym_sphinx::PAYLOAD_OVERHEAD_SIZE;

    /// Build a fresh SURB ACK addressed to `recipient` with unique `packet_id`.
    ///
    /// Samples a 3-hop route from `directory`, draws per-hop Sphinx delays using
    /// `Ts::generate_mix_delay`, and constructs a Sphinx packet whose payload is
    /// `MARKER || packet_id.to_le_bytes()`.
    pub fn construct<R>(
        rng: &mut R,
        recipient: DirectoryClient,
        packet_id: u64,
        directory: &Directory,
    ) -> Self
    where
        R: Rng,
    {
        let route = directory
            .random_route(3, rng, None)
            .into_iter()
            .collect::<Vec<_>>();
        // SAFETY : We just sampled 3 nodes, the vec isn't empty
        #[allow(clippy::unwrap_used)]
        let first_hop_id = route.first().unwrap().id;
        let sphinx_route = route
            .into_iter()
            .map(|n| n.as_sphinx_node_socket())
            .collect::<Vec<_>>();

        let destination = recipient.as_sphinx_destination();

        let delays = (0..sphinx_route.len())
            .map(|_| Delay::new_from_millis(helpers::generate_mix_delay(rng)))
            .collect::<Vec<_>>();

        let ack_payload = Self::MARKER
            .iter()
            .copied()
            .chain(packet_id.to_le_bytes())
            .collect::<Vec<_>>();

        let builder = SphinxPacketBuilder::new().with_payload_size(Self::PAYLOAD_SIZE);

        // SAFETY : We're living in a simulation, if it crashes, it crashes
        #[allow(clippy::unwrap_used)]
        let surb_ack_packet = builder
            .build_packet(ack_payload, &sphinx_route, &destination, &delays)
            .unwrap()
            .to_bytes();

        // in our case, the last hop is a gateway that does NOT do any delays
        let expected_total_delay = delays.iter().take(delays.len() - 1).sum();

        SurbAck {
            surb_ack_packet: surb_ack_packet.into(),
            first_hop_id,
            expected_total_delay,
        }
    }

    /// Byte length of a serialised SURB ACK as prepended to outgoing payloads.
    ///
    /// Format: `first_hop_id (1 byte) || sphinx_header || ack_payload`.
    pub const fn len() -> usize {
        Self::PAYLOAD_SIZE + nym_sphinx::HEADER_SIZE + 1 // SURB_FIRST_HOP || SURB_ACK
    }

    /// Return the sum of per-hop delays embedded in the SURB packet header.
    ///
    /// The terminal (gateway) hop is excluded because it applies no mix delay in
    /// the simulation.
    pub fn expected_total_delay(&self) -> Delay {
        self.expected_total_delay
    }

    /// Serialise the SURB ACK into the wire format prepended to outgoing packets.
    ///
    /// Returns `(total_delay, first_hop_id || sphinx_packet_bytes)`.  The caller
    /// hands the byte vector to the reliability layer and the delay to the
    /// scheduler.
    pub fn prepare_for_sending(self) -> (Delay, Vec<u8>) {
        // SURB_FIRST_HOP || SURB_ACK
        let surb_bytes: Vec<_> = std::iter::once(self.first_hop_id)
            .chain(self.surb_ack_packet.to_bytes())
            .collect();
        (self.expected_total_delay, surb_bytes)
    }

    /// Recover the first-hop node ID and the Sphinx ACK packet from the raw bytes
    /// produced by [`prepare_for_sending`](Self::prepare_for_sending).
    ///
    /// This is the partial inverse of `prepare_for_sending`, performed by the
    /// gateway (final-hop node) when it dispatches the SURB back into the network.
    pub fn try_recover_first_hop_packet(b: &[u8]) -> anyhow::Result<(NodeId, SimMixPacket)> {
        let first_hop_id = b[0];
        let packet = SimMixPacket::try_from_bytes(&b[1..])?;

        Ok((first_hop_id, packet))
    }

    /// Split a final-hop plaintext into `(surb_ack_bytes, message_bytes)`.
    ///
    /// If `extracted_data` is shorter than [`SurbAck::len`] (e.g. cover-traffic
    /// packets carry no SURB), the ACK slice is empty and the full buffer is
    /// returned as the message.
    pub fn extract_ack_and_message(mut extracted_data: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
        let ack_len = SurbAck::len();

        if extracted_data.len() < ack_len {
            // No SURB Ack in packet, in the sim this will be the case for cover traffic
            return (Vec::new(), extracted_data);
        }

        let message = extracted_data.split_off(ack_len);
        let ack_data = extracted_data;
        (ack_data, message)
    }

    /// Return `true` if `data` starts with the [`MARKER`](SurbAck::MARKER) bytes.
    pub fn is_surb_ack(data: &[u8]) -> bool {
        if data.len() < Self::MARKER.len() {
            return false;
        }

        data[..Self::MARKER.len()] == *Self::MARKER
    }
}
