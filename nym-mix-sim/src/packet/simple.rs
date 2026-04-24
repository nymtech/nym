// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;
use std::fmt::Debug;

use nym_common::debug::format_debug_bytes;
use nym_lp_data::{
    TimedData, TimedPayload,
    clients::traits::{Chunking, Obfuscation, ProcessingPipeline, Reliability, RoutingSecurity},
    common::traits::{Framing, Transport},
    mixnodes::traits::MixnodeProcessingPipeline,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{packet::WirePacketFormat, topology::directory::NodeId};

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
    id: Uuid,

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
                "Length mismatch to deserialize a Payload : Expected {}, got {}",
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

/// Stub client processing pipeline for [`SimplePacket`].
///
/// A no-op pass-through: returns the payload as a single packet with no
/// Sphinx layering, chunking, reliability encoding, or obfuscation.
///
/// All required sub-traits of [`ProcessingPipeline`] are implemented here;
/// [`DynProcessingPipeline`] is then provided automatically via the blanket
/// impl in `nym_lp_data`.
pub struct SimpleClientPipeline;

impl<Ts: Clone> Chunking<Ts> for SimpleClientPipeline {
    /// Returns the entire input as a single chunk, ignoring `chunk_size`.
    fn chunked(&self, input: Vec<u8>, _chunk_size: usize, timestamp: Ts) -> Vec<TimedPayload<Ts>> {
        vec![TimedData::new(timestamp, input)]
    }
}

impl<Ts> Reliability<Ts> for SimpleClientPipeline {
    const OVERHEAD_SIZE: usize = 0;
    fn reliable_encode(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        input
    }
}

impl<Ts: Clone> Obfuscation<Ts> for SimpleClientPipeline {
    fn obfuscate(&mut self, input: TimedPayload<Ts>, _timestamp: Ts) -> Vec<TimedPayload<Ts>> {
        vec![input]
    }
    fn buffer_size(&self) -> usize {
        0
    }
}

impl<Ts> RoutingSecurity<Ts> for SimpleClientPipeline {
    const OVERHEAD_SIZE: usize = 0;
    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        input
    }
}

/// Frame type is `Vec<u8>` (raw bytes); no framing overhead.
impl<Ts: Clone> Framing<Ts, Vec<u8>> for SimpleClientPipeline {
    const OVERHEAD_SIZE: usize = 0;
    fn to_frame(
        &self,
        payload: TimedPayload<Ts>,
        _frame_size: usize,
    ) -> Vec<TimedData<Ts, Vec<u8>>> {
        vec![payload]
    }
}

/// Transport wraps a raw-byte frame into a [`SimplePacket`].
/// Overhead = 16 bytes (UUID), so effective payload = 48 bytes.
impl<Ts: Clone> Transport<Ts, Vec<u8>, SimplePacket> for SimpleClientPipeline {
    const OVERHEAD_SIZE: usize = 16;
    fn to_transport_packet(&self, frame: TimedData<Ts, Vec<u8>>) -> TimedData<Ts, SimplePacket> {
        let mut padded = [0u8; 48];
        let len = frame.data.len().min(48);
        padded[..len].copy_from_slice(&frame.data[..len]);
        TimedData::new(frame.timestamp, SimplePacket::new(padded))
    }
}

impl<Ts: Clone> ProcessingPipeline<Ts, Vec<u8>, SimplePacket> for SimpleClientPipeline {
    fn packet_size(&self) -> usize {
        64
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// A pass-through [`MixnodeProcessingPipeline`] for [`SimplePacket`].
///
/// Forwards every packet unchanged to a fixed `next_hop` node.  Used in
/// `main.rs` as the placeholder pipeline until real Sphinx processing is wired
/// in.
pub struct SimplePassThroughPipeline {
    id: NodeId,
}

impl SimplePassThroughPipeline {
    pub fn new(id: NodeId) -> Self {
        Self { id }
    }
}

impl<Ts: Clone> MixnodeProcessingPipeline<Ts, SimplePacket, NodeId> for SimplePassThroughPipeline {
    fn process(
        &mut self,
        input: TimedData<Ts, SimplePacket>,
        _timestamp: Ts,
    ) -> Vec<(NodeId, TimedData<Ts, SimplePacket>)> {
        vec![(self.id + 1, input)]
    }
}
