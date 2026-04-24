// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use nym_common::debug::format_debug_bytes;
use nym_lp_data::{
    TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::packet::WirePacketFormat;

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
