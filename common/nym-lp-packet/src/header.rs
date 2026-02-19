// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MalformedLpPacketError;
use crate::message::MessageType;
use crate::version;
use bytes::{BufMut, BytesMut};
use tracing::warn;

/// Outer header (12 bytes) - always cleartext, used for routing.
///
/// This is the first 12 bytes of every LP packet, containing only the fields
/// needed for session lookup (receiver_idx) and replay protection (counter).
/// For encrypted packets, this is the AAD (additional authenticated data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OuterHeader {
    pub receiver_idx: u32,
    pub counter: u64,
}

impl OuterHeader {
    pub const SIZE: usize = 12; // receiver_idx(4) + counter(8)

    pub fn new(receiver_idx: u32, counter: u64) -> Self {
        Self {
            receiver_idx,
            counter,
        }
    }

    pub fn parse(src: &[u8]) -> Result<Self, MalformedLpPacketError> {
        if src.len() < Self::SIZE {
            return Err(MalformedLpPacketError::InsufficientData);
        }
        #[allow(clippy::unwrap_used)]
        Ok(Self {
            receiver_idx: u32::from_le_bytes(src[0..4].try_into().unwrap()),
            counter: u64::from_le_bytes(src[4..12].try_into().unwrap()),
        })
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.receiver_idx.to_le_bytes());
        bytes[4..12].copy_from_slice(&self.counter.to_le_bytes());
        bytes
    }

    /// Encode directly into a BytesMut buffer
    pub fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.receiver_idx.to_le_bytes());
        dst.put_slice(&self.counter.to_le_bytes());
    }
}

/// InnerHeader header (8 bytes) - encrypted, used for message parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InnerHeader {
    pub protocol_version: u8,
    pub reserved: [u8; 3],
    pub message_type: MessageType,
}

impl InnerHeader {
    pub const SIZE: usize = 8; // protocol_version(1) + reserved(3) + message_type(4)

    pub fn encode(&self, dst: &mut BytesMut) {
        // protocol version
        dst.put_u8(self.protocol_version);

        // reserved
        dst.put_slice(&self.reserved);

        // message type
        dst.put_slice(&(self.message_type as u32).to_le_bytes());
    }

    pub fn parse(src: &[u8]) -> Result<Self, MalformedLpPacketError> {
        if src.len() < Self::SIZE {
            return Err(MalformedLpPacketError::InsufficientData);
        }

        let protocol_version = src[0];

        // Ensure we are using compatible protocol
        // right now only support a single version
        if protocol_version > version::CURRENT {
            return Err(MalformedLpPacketError::IncompatibleFuturePacketVersion {
                got: protocol_version,
                highest_supported: version::CURRENT,
            });
        }

        if protocol_version < version::CURRENT {
            return Err(MalformedLpPacketError::IncompatibleLegacyPacketVersion {
                got: protocol_version,
                lowest_supported: version::CURRENT,
            });
        }

        // skip reserved bytes, but log if they're different from the expected zeroes
        let reserved = [src[1], src[2], src[3]];
        if reserved != [0u8; 3] {
            warn!("received non-zero reserved bytes. got: {reserved:?}");
        }

        let msg_type_raw = u32::from_le_bytes([src[4], src[5], src[6], src[7]]);
        let message_type = MessageType::from_u32(msg_type_raw)
            .ok_or_else(|| MalformedLpPacketError::invalid_message_type(msg_type_raw))?;

        Ok(InnerHeader {
            protocol_version,
            reserved,
            message_type,
        })
    }
}

/// Internal LP header representation containing all logical header fields.
///
/// **Note**: This struct represents the LOGICAL header, not the wire format.
/// On the wire, packets use the unified format where:
/// - `OuterHeader` (receiver_idx + counter) always comes first (12 bytes, cleartext)
/// - Inner content (version + reserved + payload) follows (cleartext or encrypted)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LpHeader {
    pub outer: OuterHeader,
    pub inner: InnerHeader,
}

impl LpHeader {
    pub fn new(
        receiver_idx: u32,
        counter: u64,
        protocol_version: u8,
        message_type: MessageType,
    ) -> Self {
        Self {
            outer: OuterHeader {
                receiver_idx,
                counter,
            },
            inner: InnerHeader {
                protocol_version,
                reserved: [0u8; 3],
                message_type,
            },
        }
    }

    pub(crate) fn dbg_encode(&self, dst: &mut BytesMut) {
        self.outer.encode(dst);
        self.inner.encode(dst);
    }

    /// Get the counter value from the header
    pub fn counter(&self) -> u64 {
        self.outer.counter
    }

    /// Get the sender index from the header
    pub fn receiver_idx(&self) -> u32 {
        self.outer.receiver_idx
    }
}
