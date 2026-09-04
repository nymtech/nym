// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{convert::Infallible, fmt::Debug, time::Instant};

use nym_common::debug::format_debug_bytes;
use nym_lp_data::{
    AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload,
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
    const UUID_SIZE: usize = 16;

    /// Create a new [`SimplePacket`] with a freshly generated UUID v4 and the
    /// provided 48-byte payload.
    ///
    /// The payload array is exactly `SIZE - 16 = 48` bytes so that the packet
    /// serialises to exactly [`SimplePacket::SIZE`] bytes.
    pub fn new(data: [u8; Self::SIZE - Self::UUID_SIZE]) -> Self {
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
        let uuid = Uuid::from_bytes_le(bytes[0..Self::UUID_SIZE].try_into().unwrap());
        let data = bytes[Self::UUID_SIZE..Self::SIZE].to_vec();
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

/// Intermediate frame type used by the simple client pipeline.
///
/// A `SimpleFrame` wraps a chunk of payload bytes with a fixed 7-byte magic
/// header (`b"0FRAME0"`).  It is produced by the [`Framing`] stage and
/// consumed by the [`Transport`] stage, which packs it into a [`SimplePacket`].
#[derive(Debug)]
pub struct SimpleFrame {
    pub data: Vec<u8>,
}

impl SimpleFrame {
    /// Magic header prepended to every serialised frame.
    pub const HEADER: &[u8; 7] = b"0FRAME0";

    /// Serialise the frame: magic header followed by the payload bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(Self::HEADER);
        bytes.extend_from_slice(&self.data);

        bytes
    }

    /// Deserialise a [`SimpleFrame`] by stripping the leading magic header.
    ///
    /// # Errors
    ///
    /// Returns an error if `bytes` is shorter than the 7-byte header.
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

// ─────────────────────────────────────────────────────────────────────────────
// Building blocks

/// Wrapping building block: `SimpleFrame` → `SimplePacket`.
///
/// Implements [`Framing`], [`Transport`], and [`WireWrappingPipeline`] for the
/// `SimpleFrame`/`SimplePacket` pair in one place.  Compose this into any
/// pipeline that needs wire-wrapping by delegating to `SimpleWireWrapper`.
pub struct SimpleWireWrapper;

impl Framing<()> for SimpleWireWrapper {
    type Frame = SimpleFrame;
    const OVERHEAD_SIZE: usize = SimpleFrame::HEADER.len();
    fn to_frame(
        &mut self,
        payload: AddressedTimedPayload,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<SimpleFrame>> {
        payload
            .data
            .data
            .chunks(frame_size)
            .map(|chunk| {
                AddressedTimedData::new_addressed(
                    payload.data.timestamp,
                    SimpleFrame {
                        data: chunk.to_vec(),
                    },
                    payload.dst,
                )
            })
            .collect()
    }
}

/// Transport wraps a [`SimpleFrame`] into a [`SimplePacket`].
/// Overhead = 16 bytes (UUID), so effective payload = 48 bytes.
impl Transport<SimplePacket> for SimpleWireWrapper {
    type Frame = SimpleFrame;
    type Error = Infallible;
    const OVERHEAD_SIZE: usize = 16;
    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<SimpleFrame>,
    ) -> Result<AddressedTimedData<SimplePacket>, Self::Error> {
        // SAFETY: If the pipeline is implemented properly, frames perfectly fit in a packet
        #[allow(clippy::unwrap_used)]
        Ok(frame.data_transform(|inner| SimplePacket::new(inner.to_bytes().try_into().unwrap())))
    }
}

impl WireWrappingPipeline<SimplePacket, ()> for SimpleWireWrapper {
    fn packet_size(&self) -> usize {
        SimplePacket::SIZE
    }
}

/// Unwrapping building block: `SimplePacket` → payload.
///
/// Implements [`TransportUnwrap`], [`FramingUnwrap`], and
/// [`WireUnwrappingPipeline`] for the `SimpleFrame`/`SimplePacket` pair.
/// Compose into any pipeline that needs frame-unwrapping by delegating to
/// `SimpleWireUnwrapper`.
pub struct SimpleWireUnwrapper;

impl FramingUnwrap<()> for SimpleWireUnwrapper {
    type Frame = SimpleFrame;
    fn frame_to_message(&mut self, frame: TimedData<SimpleFrame>) -> Option<(TimedPayload, ())> {
        Some((
            TimedPayload {
                data: frame.data.data,
                timestamp: frame.timestamp,
            },
            (),
        ))
    }
}

impl TransportUnwrap<SimplePacket> for SimpleWireUnwrapper {
    type Frame = SimpleFrame;
    type Error = anyhow::Error;
    fn packet_to_frame(
        &mut self,
        packet: SimplePacket,
        timestamp: Instant,
    ) -> anyhow::Result<TimedData<SimpleFrame>> {
        // packet.data holds the framed bytes (HEADER + payload)
        Ok(TimedData::new(
            timestamp,
            SimpleFrame::try_from_bytes(&packet.data)?,
        ))
    }
}

impl WireUnwrappingPipeline<SimplePacket, ()> for SimpleWireUnwrapper {}
