// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Packet types and the generic wire-format trait used by the simulation.
//!
//! The central abstraction is [`WirePacketFormat<Ts>`]: a trait that any packet
//! type must implement to participate in a simulation.  The `Ts` type parameter
//! represents a *timestamp* (or more generally, any per-tick context) that is
//! threaded through the [`WirePacketFormat::process`] call so that mix
//! operations can be timestamp-aware if needed.
//!
//! [`SimplePacket`] is a built-in concrete implementation: a fixed-size 64-byte
//! packet (16-byte UUID + 48-byte payload)

use std::fmt;
use std::fmt::Debug;

use nym_common::debug::format_debug_bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        // simple length prefixed serialization
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
///
/// The timestamp type `Ts` is unused — the mix operation is purely a function
/// of the payload bytes and does not depend on when the packet was processed.
impl<Ts> WirePacketFormat<Ts> for SimplePacket {
    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Self::try_from_bytes(bytes)
    }

    /// Dummy processing
    fn process(mut self, _: Ts) -> anyhow::Result<Self> {
        self.data = self.data.into_iter().map(|b| b + 1).collect();
        Ok(self)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

/// Trait that every packet type must implement to participate in the simulation.
///
/// The type parameter `Ts` is a *timestamp* (or any per-tick context value)
/// that is passed to [`process`] so that mix operations can be aware of the
/// current simulation time if needed.
///
/// ## Bounds
///
/// * `Debug` — required so that [`crate::node::Node::display_state`] can print
///   packet buffers without knowing the concrete type.
/// * `Sized` — required because the trait is used with `Vec<Pkt>` and moved by
///   value in several places.
/// * `Send + 'static` — required because the simulation driver spawns a
///   `tokio::task` that owns the node list.
///
pub trait WirePacketFormat<Ts>: Debug + Sized + Send + 'static {
    /// Deserialise a packet from the raw bytes received off the wire.
    ///
    /// # Errors
    ///
    /// Should return an error on length mismatch, invalid magic bytes, or any
    /// other malformed-datagram condition.
    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self>;

    /// Apply the node's mix operation to the packet and return the result.
    ///
    /// `timestamp` carries the current tick's context value, used for cover
    /// traffic generation, time-based batching, and delay policies.
    ///
    /// # Errors
    ///
    /// Should return an error if the packet cannot be processed (e.g. decryption
    /// failure, unsupported version).
    fn process(self, timestamp: Ts) -> anyhow::Result<Self>;

    /// Serialise the packet to its on-wire byte representation, ready to be
    /// sent via UDP.
    fn to_bytes(&self) -> Vec<u8>;
}
