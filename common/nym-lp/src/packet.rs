// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpError;
use crate::message::{LpMessage, MessageType};
use crate::replay::ReceivingKeyCounterValidator;
use bytes::{BufMut, BytesMut};
use nym_lp_common::format_debug_bytes;
use std::fmt::{Debug, Formatter};
use tracing::warn;

#[allow(dead_code)]
pub(crate) const UDP_HEADER_LEN: usize = 8;
#[allow(dead_code)]
pub(crate) const IP_HEADER_LEN: usize = 40; // v4 - 20, v6 - 40
#[allow(dead_code)]
pub(crate) const MTU: usize = 1500;
#[allow(dead_code)]
pub(crate) const UDP_OVERHEAD: usize = UDP_HEADER_LEN + IP_HEADER_LEN;

#[allow(dead_code)]
pub const TRAILER_LEN: usize = 16;
#[allow(dead_code)]
pub(crate) const UDP_PAYLOAD_SIZE: usize = MTU - UDP_OVERHEAD - TRAILER_LEN;

pub mod version {
    /// The current version of the Lewes Protocol that is put into each new constructed header.
    pub const CURRENT: u8 = 1;
}

#[derive(Clone)]
pub struct EncryptedLpPacket {
    // The outer header that's sent in plaintext
    pub(crate) outer_header: OuterHeader,

    // The ciphertext containing the inner header and the payload
    pub(crate) ciphertext: Vec<u8>,
}

impl Debug for EncryptedLpPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_debug_bytes(&self.debug_bytes())?)
    }
}

impl EncryptedLpPacket {
    pub(crate) fn debug_bytes(&self) -> Vec<u8> {
        let mut bytes = BytesMut::new();
        self.encode(&mut bytes);
        bytes.freeze().to_vec()
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        self.outer_header.encode(dst);
        dst.put_slice(&self.ciphertext)
    }
}

#[derive(Clone)]
pub struct LpPacket {
    pub(crate) header: LpHeader,
    pub(crate) message: LpMessage,
}

impl Debug for LpPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_debug_bytes(&self.debug_bytes())?)
    }
}

impl LpPacket {
    pub fn new(header: LpHeader, message: LpMessage) -> Self {
        Self { header, message }
    }

    pub fn typ(&self) -> MessageType {
        self.message.typ()
    }

    pub fn message(&self) -> &LpMessage {
        &self.message
    }

    pub fn header(&self) -> &LpHeader {
        &self.header
    }

    pub(crate) fn debug_bytes(&self) -> Vec<u8> {
        let mut bytes = BytesMut::new();
        self.dbg_encode(&mut bytes);
        bytes.freeze().to_vec()
    }

    pub(crate) fn dbg_encode(&self, dst: &mut BytesMut) {
        self.header.dbg_encode(dst);

        dst.put_slice(&(self.message.typ() as u16).to_le_bytes());
        self.message.encode_content(dst);
    }

    /// Validate packet counter against a replay protection validator
    ///
    /// This performs a quick check to see if the packet counter is valid before
    /// any expensive processing is done.
    pub fn validate_counter(
        &self,
        validator: &ReceivingKeyCounterValidator,
    ) -> Result<(), LpError> {
        validator.will_accept_branchless(self.header.outer.counter)?;
        Ok(())
    }

    /// Mark packet as received in the replay protection validator
    ///
    /// This should be called after a packet has been successfully processed.
    pub fn mark_received(
        &self,
        validator: &mut ReceivingKeyCounterValidator,
    ) -> Result<(), LpError> {
        validator.mark_did_receive_branchless(self.header.outer.counter)?;
        Ok(())
    }
}

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

    pub fn parse(src: &[u8]) -> Result<Self, LpError> {
        if src.len() < Self::SIZE {
            return Err(LpError::InsufficientBufferSize);
        }
        Ok(Self {
            receiver_idx: u32::from_le_bytes(src[0..4].try_into().unwrap()),
            counter: u64::from_le_bytes(src[4..12].try_into().unwrap()),
        })
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

    pub(crate) fn encode(&self, dst: &mut BytesMut) {
        // protocol version
        dst.put_u8(self.protocol_version);

        // reserved
        dst.put_slice(&self.reserved);

        // message type
        dst.put_slice(&(self.message_type as u32).to_le_bytes());
    }

    pub(crate) fn parse(src: &[u8]) -> Result<Self, LpError> {
        if src.len() < Self::SIZE {
            return Err(LpError::InsufficientBufferSize);
        }

        let protocol_version = src[0];

        // Ensure we are using compatible protocol
        // right now only support a single version
        if protocol_version > version::CURRENT {
            return Err(LpError::IncompatibleFuturePacketVersion {
                got: protocol_version,
                highest_supported: version::CURRENT,
            });
        }

        if protocol_version < version::CURRENT {
            return Err(LpError::IncompatibleLegacyPacketVersion {
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
            .ok_or_else(|| LpError::invalid_message_type(msg_type_raw))?;

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

// subsequent data: MessageType || Data
