// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_common::debug::format_debug_bytes;
use nym_lp_data::{
    AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};
use nym_sphinx::{Delay, Destination, DestinationAddressBytes, SphinxPacketBuilder};

use rand::Rng;
use rand_distr::{Distribution, Exp};
use std::{fmt::Debug, ops::Add, time::Duration};

use crate::{
    client::ClientId, node::NodeId, packet::WirePacketFormat, topology::directory::Directory,
};

/// Newtype wrapper that provides a trimmed [`Debug`]
/// implementation (showing only the first 32 bytes of the serialised form to
/// avoid flooding logs).
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
    pub fn construct<Ts: GenerateDelay, R>(
        rng: &mut R,
        recipient: ClientId,
        packet_id: u64,
        directory: &Directory,
    ) -> Self
    where
        R: Rng,
    {
        let route = directory
            .random_route(3, rng)
            .into_iter()
            .collect::<Vec<_>>();
        // SAFETY : We just sampled 3 nodes, the vec isn't empty
        #[allow(clippy::unwrap_used)]
        let first_hop_id = route.first().unwrap().id;
        let sphinx_route = route.into_iter().map(Into::into).collect::<Vec<_>>();

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([recipient; 32]),
            [recipient; 16],
        );

        let delays = (0..sphinx_route.len())
            .map(|_| Delay::new_from_millis(Ts::generate_mix_delay(rng)))
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
    /// produced by [`prepare_for_sending`].
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

/// Marker type identifying a fully-unwrapped Sphinx payload.
///
/// Passed through the pipeline's [`FramingUnwrap`] stage so that
/// [`ClientUnwrappingPipeline::process_unwrapped`] can dispatch on the message
/// kind.  In the simulation there is only one kind, so this is a zero-sized
/// unit struct.
///
/// [`FramingUnwrap`]: nym_lp_data::common::traits::FramingUnwrap
/// [`ClientUnwrappingPipeline::process_unwrapped`]: nym_lp_data::clients::traits::ClientUnwrappingPipeline::process_unwrapped
pub struct SphinxMessage;

/// Abstracts adding a Sphinx [`Delay`](nym_sphinx::Delay) to a timestamp type.
pub trait AddDelay: Sized {
    fn add_delay(self, delay: nym_sphinx::Delay) -> Self;
}

impl AddDelay for u32 {
    /// One tick = 1 ms.
    fn add_delay(self, delay: nym_sphinx::Delay) -> Self {
        self + (delay.to_nanos() / 1_000_000) as u32
    }
}

impl AddDelay for std::time::Instant {
    fn add_delay(self, delay: nym_sphinx::Delay) -> Self {
        self + delay.to_duration()
    }
}

/// Timestamp types that can generate Sphinx delays and be advanced by them.
///
/// Implemented for `u32` (discrete ticks, 1 tick = 1 ms) and [`Instant`]
/// (wall-clock time).
pub trait GenerateDelay: Sized + Add<Self::Delay, Output = Self> {
    /// The delay unit that can be added to `Self` (e.g. `u32` ticks or
    /// [`Duration`](std::time::Duration)).
    type Delay;

    /// Draw a per-hop mix delay in milliseconds for inclusion in a Sphinx packet header.
    fn generate_mix_delay(rng: &mut impl Rng) -> u64;

    /// Draw an inter-packet sending delay for the main Poisson loop.
    fn generate_sending_delay(rng: &mut impl Rng) -> Self::Delay;

    /// Draw an inter-packet sending delay for the secondary cover traffic loop.
    fn generate_cover_traffic_delay(rng: &mut impl Rng) -> Self::Delay;
}

impl GenerateDelay for u32 {
    type Delay = u32;

    /// Uniform in `[0, 10]` ms.
    fn generate_mix_delay(rng: &mut impl Rng) -> u64 {
        rng.gen_range(0..=10)
    }

    /// Exponential with mean 10 ticks (ms).
    fn generate_sending_delay(rng: &mut impl Rng) -> u32 {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 10.0).unwrap();
        exp.sample(rng).round() as u32
    }

    /// Exponential with mean 100 ticks (ms).
    fn generate_cover_traffic_delay(rng: &mut impl Rng) -> u32 {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 100.0).unwrap();
        exp.sample(rng).round() as u32
    }
}

impl GenerateDelay for std::time::Instant {
    type Delay = Duration;

    /// Exponential with mean 50 ms.
    fn generate_mix_delay(rng: &mut impl Rng) -> u64 {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 50.0).unwrap();
        exp.sample(rng).round() as u64
    }

    /// Exponential with mean 20 ms.
    fn generate_sending_delay(rng: &mut impl Rng) -> Duration {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 20.0).unwrap();
        Duration::from_millis(exp.sample(rng).round() as u64)
    }

    /// Exponential with mean 200 ms.
    fn generate_cover_traffic_delay(rng: &mut impl Rng) -> Duration {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 200.0).unwrap();
        Duration::from_millis(exp.sample(rng).round() as u64)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Building blocks

/// Wrapping building block: `SphinxPacket` → `SphinxPacket`.
///
/// Passthrough wrapper and unwrapper for sphinx compatibility demonstration
pub struct SphinxNoOpWireWrapper;

impl<Ts: Clone> Framing<Ts, NodeId> for SphinxNoOpWireWrapper {
    type Frame = Vec<u8>;
    const OVERHEAD_SIZE: usize = 0;
    fn to_frame(
        &self,
        payload: AddressedTimedPayload<Ts, NodeId>,
        _frame_size: usize,
    ) -> Vec<AddressedTimedPayload<Ts, NodeId>> {
        vec![payload]
    }
}

impl<Ts: Clone> Transport<Ts, SimMixPacket, NodeId> for SphinxNoOpWireWrapper {
    type Frame = Vec<u8>;
    const OVERHEAD_SIZE: usize = 0;
    fn to_transport_packet(
        &self,
        frame: AddressedTimedPayload<Ts, NodeId>,
    ) -> AddressedTimedData<Ts, SimMixPacket, NodeId> {
        frame.data_transform(SimMixPacket)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SimMixPacket, NodeId> for SphinxNoOpWireWrapper {
    fn packet_size(&self) -> usize {
        1500
    }
}

/// Unwrapping building block: `SimSphinxPacket` → `SphinxPacket`.
///
/// The inverse of [`SphinxNoOpWireWrapper`]: unwraps the [`SimSphinxPacket`]
/// newtype to the inner [`SphinxPacket`] and serialises it back to bytes for
/// the downstream [`FramingUnwrap`] stage.
///
/// [`FramingUnwrap`]: nym_lp_data::common::traits::FramingUnwrap
pub struct SphinxNoOpWireUnwrapper;

impl<Ts> FramingUnwrap<Ts, SphinxMessage> for SphinxNoOpWireUnwrapper {
    type Frame = Vec<u8>;
    fn frame_to_message(
        &mut self,
        frame: TimedPayload<Ts>,
    ) -> Option<(TimedPayload<Ts>, SphinxMessage)> {
        Some((frame, SphinxMessage))
    }
}

impl<Ts: Clone> TransportUnwrap<Ts, SimMixPacket> for SphinxNoOpWireUnwrapper {
    type Frame = Vec<u8>;
    fn packet_to_frame(
        &self,
        packet: SimMixPacket,
        timestamp: Ts,
    ) -> anyhow::Result<TimedPayload<Ts>> {
        Ok(TimedData {
            timestamp,
            data: packet.0,
        })
    }
}

impl<Ts: Clone> WireUnwrappingPipeline<Ts, SimMixPacket, SphinxMessage> for SphinxNoOpWireUnwrapper {}
