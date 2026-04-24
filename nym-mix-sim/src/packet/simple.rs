// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;
use std::fmt::Debug;

use nym_common::debug::format_debug_bytes;
use nym_lp_data::{
    TimedData, TimedPayload,
    clients::{
        helpers::{NoOpObfuscation, NoOpReliability, NoOpRoutingSecurity},
        traits::{Chunking, ClientUnwrappingPipeline, ClientWrappingPipeline},
    },
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
    mixnodes::traits::MixnodeProcessingPipeline,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{node::NodeId, packet::WirePacketFormat};

/// A minimal, fixed-size packet used by the simulation.
///
/// ## Wire format
///
/// ```text
/// ┌──────────────────┬──────────────────────────────────────────────────┐
/// │  UUID (16 bytes) │              payload (48 bytes)                  │
/// │  little-endian   │                                                  │
/// └──────────────────┴──────────────────────────────────────────────────┘
///  byte 0            16                                                64
/// ```
///
/// The total on-wire size is always exactly [`SimplePacket::SIZE`] = 64 bytes.
#[derive(Serialize, Deserialize)]
pub struct SimplePacket {
    /// Universally unique identifier assigned at creation time (UUID v4).
    /// Used to correlate a packet across hops for debugging and tracing.
    pub id: Uuid,

    /// Variable-length payload buffer.
    ///
    /// Despite the type being `Vec<u8>`, the simulation always creates and
    /// expects exactly 48 bytes here (i.e. `SIZE - 16`).  The `Vec` is used
    /// rather than a fixed array to keep serialisation simple.
    pub data: Vec<u8>,
}

impl Debug for SimplePacket {
    /// Pretty-prints the packet ID followed by a hex dump of the payload via
    /// [`format_debug_bytes`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SimplePacket {{")?;
        writeln!(f, "    id: {:?},", self.id)?;
        writeln!(f, "    data:")?;
        for line in format_debug_bytes(&self.data)?.lines() {
            writeln!(f, "        {line}")?;
        }
        write!(f, "}}")
    }
}

impl SimplePacket {
    /// On-wire size of a serialised [`SimplePacket`] in bytes.
    ///
    /// Layout: 16 bytes UUID (LE) + 48 bytes payload = 64 bytes total.
    const SIZE: usize = 64;

    /// Create a new [`SimplePacket`] with a freshly generated UUID v4 and the
    /// provided 48-byte payload.
    ///
    /// The payload array is exactly `SIZE - 16 = 48` bytes so that the packet
    /// serialises to exactly [`SimplePacket::SIZE`] bytes.
    pub fn new(data: [u8; Self::SIZE - 16]) -> Self {
        Self {
            id: Uuid::new_v4(),
            data: data.to_vec(),
        }
    }

    /// Return the packet's UUID identifier.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Return a clone of the raw payload bytes.
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Serialise the packet to its fixed-size wire representation.
    ///
    /// Layout: UUID as 16 little-endian bytes, followed by the 48-byte payload.
    /// The returned `Vec` is always exactly [`SimplePacket::SIZE`] bytes long.
    pub fn to_bytes(&self) -> Vec<u8> {
        // fixed-size serialization: 16-byte UUID followed by 48-byte payload
        let mut bytes = Vec::with_capacity(Self::SIZE);

        bytes.extend_from_slice(&self.id.to_bytes_le()); // 16 bytes
        bytes.extend_from_slice(&self.data); // 48 bytes

        bytes
    }

    /// Deserialise a [`SimplePacket`] from a raw byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if `bytes.len() != SIZE` (64).  Any other slice length
    /// indicates a truncated or corrupted UDP datagram.
    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != Self::SIZE {
            return Err(anyhow::anyhow!(
                "Length mismatch to deserialize a SimplePacket : Expected {}, got {}",
                Self::SIZE,
                bytes.len()
            ));
        }
        #[allow(clippy::unwrap_used)]
        let uuid = Uuid::from_bytes_le(bytes[0..16].try_into().unwrap());
        let data = bytes[16..Self::SIZE].to_vec();
        Ok(SimplePacket { id: uuid, data })
    }
}

/// [`WirePacketFormat`] implementation for [`SimplePacket`].
impl WirePacketFormat for SimplePacket {
    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Self::try_from_bytes(bytes)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

/// Intermediate frame type used by [`SimpleClientPipeline`].
///
/// A `SimpleFrame` wraps a chunk of payload bytes with a fixed 7-byte magic
/// header (`b"0FRAME0"`).  It is produced by the [`Framing`] stage and
/// consumed by the [`Transport`] stage, which packs it into a [`SimplePacket`].
pub struct SimpleFrame {
    pub data: Vec<u8>,
}

impl SimpleFrame {
    /// Magic header prepended to every serialised frame.
    pub const HEADER: &[u8; 7] = b"0FRAME0";

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(Self::HEADER);
        bytes.extend_from_slice(&self.data);

        bytes
    }

    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() < Self::HEADER.len() {
            return Err(anyhow::anyhow!(
                "Length mismatch to deserialize a SimpleFrame : Expected at least {}, got {}",
                Self::HEADER.len(),
                bytes.len()
            ));
        }
        let data = bytes[Self::HEADER.len()..].to_vec();
        Ok(SimpleFrame { data })
    }
}

pub struct SimpleMessage;

// ─────────────────────────────────────────────────────────────────────────────
// Building blocks

/// Wrapping building block: `SimpleFrame` → `SimplePacket`.
///
/// Implements [`Framing`], [`Transport`], and [`WireWrappingPipeline`] for the
/// `SimpleFrame`/`SimplePacket` pair in one place.  Compose this into any
/// pipeline that needs wire-wrapping by delegating to `SimpleWireWrapper`.
pub struct SimpleWireWrapper;

impl<Ts: Clone> Framing<Ts, SimpleFrame> for SimpleWireWrapper {
    const OVERHEAD_SIZE: usize = SimpleFrame::HEADER.len();
    fn to_frame(
        &self,
        payload: TimedPayload<Ts>,
        frame_size: usize,
    ) -> Vec<TimedData<Ts, SimpleFrame>> {
        payload
            .data
            .chunks(frame_size)
            .map(|chunk| TimedData {
                data: SimpleFrame {
                    data: chunk.to_vec(),
                },
                timestamp: payload.timestamp.clone(),
            })
            .collect()
    }
}

/// Transport wraps a [`SimpleFrame`] into a [`SimplePacket`].
/// Overhead = 16 bytes (UUID), so effective payload = 48 bytes.
impl<Ts: Clone> Transport<Ts, SimpleFrame, SimplePacket> for SimpleWireWrapper {
    const OVERHEAD_SIZE: usize = 16;
    fn to_transport_packet(
        &self,
        frame: TimedData<Ts, SimpleFrame>,
    ) -> TimedData<Ts, SimplePacket> {
        // SAFETY: If the pipeline is implemented properly, frames perfectly fit in a packet
        #[allow(clippy::unwrap_used)]
        let packet = SimplePacket::new(frame.data.to_bytes().try_into().unwrap());
        TimedData::new(frame.timestamp, packet)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SimpleFrame, SimplePacket> for SimpleWireWrapper {
    fn packet_size(&self) -> usize {
        SimplePacket::SIZE
    }
}

/// Unwrapping building block: `SimpleFrame` → payload.
///
/// Implements [`FramingUnwrap`] and [`WireUnwrappingPipeline`] for the
/// `SimpleFrame`/`SimplePacket` pair.  Compose this into any pipeline that
/// needs frame-unwrapping by delegating to `SimpleFrameUnwrapper`.
///
/// Note: [`TransportUnwrap`] is *not* included here because the unwrap
/// direction differs between mixnode (strips the framing header) and client
/// (keeps the raw bytes as-is).
pub struct SimpleWireUnwrapper;

impl<Ts> FramingUnwrap<Ts, SimpleFrame, SimpleMessage> for SimpleWireUnwrapper {
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, SimpleFrame>,
    ) -> Option<(TimedPayload<Ts>, SimpleMessage)> {
        Some((
            TimedPayload {
                data: frame.data.data,
                timestamp: frame.timestamp,
            },
            SimpleMessage,
        ))
    }
}

impl<Ts: Clone> TransportUnwrap<Ts, SimpleFrame, SimplePacket> for SimpleWireUnwrapper {
    fn packet_to_frame(
        &self,
        packet: SimplePacket,
        timestamp: Ts,
    ) -> anyhow::Result<TimedData<Ts, SimpleFrame>> {
        // packet.data holds the framed bytes (HEADER + payload)
        Ok(TimedData::new(
            timestamp,
            SimpleFrame::try_from_bytes(&packet.data)?,
        ))
    }
}

impl<Ts: Clone> WireUnwrappingPipeline<Ts, SimpleFrame, SimplePacket, SimpleMessage>
    for SimpleWireUnwrapper
{
}

// ─────────────────────────────────────────────────────────────────────────────
// Concrete pipelines

/// Stub client processing pipeline for [`SimplePacket`].
///
/// A no-op pass-through: returns the payload as a single packet with no
/// Sphinx layering, chunking, reliability encoding, or obfuscation.
///
/// All required sub-traits of [`ClientWrappingPipeline`] are implemented here;
/// [`DynClientWrappingPipeline`] is then provided automatically via the blanket
/// impl in `nym_lp_data`.
pub struct SimpleClientWrappingPipeline(SimpleWireWrapper);

impl Default for SimpleClientWrappingPipeline {
    fn default() -> Self {
        Self(SimpleWireWrapper)
    }
}

impl<Ts: Clone> Chunking<Ts> for SimpleClientWrappingPipeline {
    fn chunked(
        &self,
        mut input: Vec<u8>,
        chunk_size: usize,
        timestamp: Ts,
    ) -> Vec<TimedPayload<Ts>> {
        // Padding with 10000...
        input.push(1);
        if !input.len().is_multiple_of(chunk_size) {
            let padding = vec![0; chunk_size - input.len() % chunk_size];
            input.extend_from_slice(&padding);
        }

        input
            .chunks(chunk_size)
            .map(|chunk| TimedData {
                data: chunk.to_vec(),
                timestamp: timestamp.clone(),
            })
            .collect()
    }
}

impl NoOpReliability for SimpleClientWrappingPipeline {}
impl NoOpObfuscation for SimpleClientWrappingPipeline {}
impl NoOpRoutingSecurity for SimpleClientWrappingPipeline {}

impl<Ts: Clone> Framing<Ts, SimpleFrame> for SimpleClientWrappingPipeline {
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Framing<Ts, _>>::OVERHEAD_SIZE;
    fn to_frame(
        &self,
        payload: TimedPayload<Ts>,
        frame_size: usize,
    ) -> Vec<TimedData<Ts, SimpleFrame>> {
        self.0.to_frame(payload, frame_size)
    }
}

impl<Ts: Clone> Transport<Ts, SimpleFrame, SimplePacket> for SimpleClientWrappingPipeline {
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Transport<Ts, _, _>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &self,
        frame: TimedData<Ts, SimpleFrame>,
    ) -> TimedData<Ts, SimplePacket> {
        self.0.to_transport_packet(frame)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SimpleFrame, SimplePacket>
    for SimpleClientWrappingPipeline
{
    fn packet_size(&self) -> usize {
        <SimpleWireWrapper as WireWrappingPipeline<Ts, SimpleFrame, SimplePacket>>::packet_size(
            &self.0,
        )
    }
}

impl<Ts: Clone> ClientWrappingPipeline<Ts, SimpleFrame, SimplePacket>
    for SimpleClientWrappingPipeline
{
}

// ─────────────────────────────────────────────────────────────────────────────

/// A simple [`MixnodeProcessingPipeline`] for [`SimplePacket`].
///
/// Demonstrates the full pipeline: unwraps the incoming packet through the
/// wire layer (transport → frame → payload), applies a routing decision in
/// [`mix`] (forwards to `self.id + 1`), then re-wraps the outgoing payload
/// (payload → frame → transport) before sending.
pub struct SimpleMixnodePipeline {
    id: NodeId,
    wrapper: SimpleWireWrapper,
    unwrapper: SimpleWireUnwrapper,
}

impl SimpleMixnodePipeline {
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            wrapper: SimpleWireWrapper,
            unwrapper: SimpleWireUnwrapper,
        }
    }
}

impl<Ts: Clone> Framing<Ts, SimpleFrame> for SimpleMixnodePipeline {
    const OVERHEAD_SIZE: usize = SimpleFrame::HEADER.len();
    fn to_frame(
        &self,
        payload: TimedPayload<Ts>,
        frame_size: usize,
    ) -> Vec<TimedData<Ts, SimpleFrame>> {
        self.wrapper.to_frame(payload, frame_size)
    }
}

impl<Ts: Clone> Transport<Ts, SimpleFrame, SimplePacket> for SimpleMixnodePipeline {
    const OVERHEAD_SIZE: usize = 16; // UUID overhead
    fn to_transport_packet(
        &self,
        frame: TimedData<Ts, SimpleFrame>,
    ) -> TimedData<Ts, SimplePacket> {
        self.wrapper.to_transport_packet(frame)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SimpleFrame, SimplePacket> for SimpleMixnodePipeline {
    fn packet_size(&self) -> usize {
        <SimpleWireWrapper as WireWrappingPipeline<Ts, SimpleFrame, SimplePacket>>::packet_size(
            &self.wrapper,
        )
    }
}

impl<Ts> FramingUnwrap<Ts, SimpleFrame, SimpleMessage> for SimpleMixnodePipeline {
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, SimpleFrame>,
    ) -> Option<(TimedPayload<Ts>, SimpleMessage)> {
        self.unwrapper.frame_to_message(frame)
    }
}

impl<Ts: Clone> TransportUnwrap<Ts, SimpleFrame, SimplePacket> for SimpleMixnodePipeline {
    fn packet_to_frame(
        &self,
        packet: SimplePacket,
        timestamp: Ts,
    ) -> anyhow::Result<TimedData<Ts, SimpleFrame>> {
        self.unwrapper.packet_to_frame(packet, timestamp)
    }
}

impl<Ts: Clone> WireUnwrappingPipeline<Ts, SimpleFrame, SimplePacket, SimpleMessage>
    for SimpleMixnodePipeline
{
}

impl<Ts: Clone> MixnodeProcessingPipeline<Ts, SimpleFrame, SimplePacket, SimpleMessage, NodeId>
    for SimpleMixnodePipeline
{
    fn mix(
        &mut self,
        payload: TimedPayload<Ts>,
        _timestamp: Ts,
    ) -> Vec<(NodeId, TimedPayload<Ts>)> {
        // Routing decision: forward to the next node
        vec![(self.id + 1, payload)]
    }
}

// ─────────────────────────────────────────────────────────────────────────────

pub struct SimpleClientUnwrapping(SimpleWireUnwrapper);

impl Default for SimpleClientUnwrapping {
    fn default() -> Self {
        Self(SimpleWireUnwrapper)
    }
}

impl<Ts> FramingUnwrap<Ts, SimpleFrame, SimpleMessage> for SimpleClientUnwrapping {
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, SimpleFrame>,
    ) -> Option<(TimedPayload<Ts>, SimpleMessage)> {
        self.0.frame_to_message(frame)
    }
}

impl<Ts: Clone> TransportUnwrap<Ts, SimpleFrame, SimplePacket> for SimpleClientUnwrapping {
    fn packet_to_frame(
        &self,
        packet: SimplePacket,
        timestamp: Ts,
    ) -> anyhow::Result<TimedData<Ts, SimpleFrame>> {
        self.0.packet_to_frame(packet, timestamp)
    }
}

impl<Ts: Clone> WireUnwrappingPipeline<Ts, SimpleFrame, SimplePacket, SimpleMessage>
    for SimpleClientUnwrapping
{
}

impl<Ts: Clone> ClientUnwrappingPipeline<Ts, SimpleFrame, SimplePacket, SimpleMessage>
    for SimpleClientUnwrapping
{
    fn process_unwrapped(
        &mut self,
        payload: TimedPayload<Ts>,
        _kind: SimpleMessage,
    ) -> Option<Vec<u8>> {
        let mut data = payload.data;
        if let Some(pos) = data.iter().rposition(|&b| b == 1) {
            data.truncate(pos);
        }
        Some(data)
    }
}
