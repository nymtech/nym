// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpError;
use crate::message::LpMessage;
use crate::replay::ReceivingKeyCounterValidator;
use bytes::{BufMut, BytesMut};
use nym_lp_common::format_debug_bytes;
use parking_lot::Mutex;
use std::fmt::Write;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
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
pub struct LpPacket {
    pub(crate) header: LpHeader,
    pub(crate) message: LpMessage,
    pub(crate) trailer: [u8; TRAILER_LEN],
}

impl Debug for LpPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_debug_bytes(&self.debug_bytes())?)
    }
}

impl LpPacket {
    pub fn new(header: LpHeader, message: LpMessage) -> Self {
        Self {
            header,
            message,
            trailer: [0; TRAILER_LEN],
        }
    }

    /// Compute a hash of the message payload
    ///
    /// This can be used for message integrity verification or deduplication
    pub fn hash_payload(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        let mut buffer = BytesMut::new();

        // Include message type and content in the hash
        buffer.put_slice(&(self.message.typ() as u16).to_le_bytes());
        self.message.encode_content(&mut buffer);

        hasher.update(&buffer);
        hasher.finalize().into()
    }

    pub fn hash_payload_hex(&self) -> String {
        let hash = self.hash_payload();
        hash.iter()
            .fold(String::with_capacity(hash.len() * 2), |mut acc, byte| {
                let _ = write!(acc, "{:02x}", byte);
                acc
            })
    }

    pub fn message(&self) -> &LpMessage {
        &self.message
    }

    pub fn header(&self) -> &LpHeader {
        &self.header
    }

    pub(crate) fn debug_bytes(&self) -> Vec<u8> {
        let mut bytes = BytesMut::new();
        self.encode(&mut bytes);
        bytes.freeze().to_vec()
    }

    pub(crate) fn encode(&self, dst: &mut BytesMut) {
        self.header.encode(dst);

        dst.put_slice(&(self.message.typ() as u16).to_le_bytes());
        self.message.encode_content(dst);

        dst.put_slice(&self.trailer)
    }

    /// Validate packet counter against a replay protection validator
    ///
    /// This performs a quick check to see if the packet counter is valid before
    /// any expensive processing is done.
    pub fn validate_counter(
        &self,
        validator: &Arc<Mutex<ReceivingKeyCounterValidator>>,
    ) -> Result<(), LpError> {
        let guard = validator.lock();
        guard.will_accept_branchless(self.header.counter)?;
        Ok(())
    }

    /// Mark packet as received in the replay protection validator
    ///
    /// This should be called after a packet has been successfully processed.
    pub fn mark_received(
        &self,
        validator: &Arc<Mutex<ReceivingKeyCounterValidator>>,
    ) -> Result<(), LpError> {
        let mut guard = validator.lock();
        guard.mark_did_receive_branchless(self.header.counter)?;
        Ok(())
    }
}

/// Session ID used for ClientHello bootstrap packets before session is established.
///
/// When a client first connects, it sends a ClientHello packet with receiver_idx=0
/// because neither side can compute the deterministic session ID yet (requires
/// both parties' X25519 keys). After ClientHello is processed, both sides derive
/// the same session ID from their keys, and all subsequent packets use that ID.
pub const BOOTSTRAP_RECEIVER_IDX: u32 = 0;

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

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..4].copy_from_slice(&self.receiver_idx.to_le_bytes());
        buf[4..12].copy_from_slice(&self.counter.to_le_bytes());
        buf
    }

    /// Encode directly into a BytesMut buffer
    pub fn encode_into(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.receiver_idx.to_le_bytes());
        dst.put_slice(&self.counter.to_le_bytes());
    }
}

/// Internal LP header representation containing all logical header fields.
///
/// **Note**: This struct represents the LOGICAL header, not the wire format.
/// On the wire, packets use the unified format where:
/// - `OuterHeader` (receiver_idx + counter) always comes first (12 bytes, cleartext)
/// - Inner content (version + reserved + payload) follows (cleartext or encrypted)
///
/// The `LpHeader::encode()` method outputs the old logical format for debug purposes only.
/// Use `serialize_lp_packet()` in codec.rs for actual wire serialization.
#[derive(Debug, Clone)]
pub struct LpHeader {
    pub protocol_version: u8,
    pub reserved: [u8; 3],
    pub receiver_idx: u32,
    pub counter: u64,
}

impl LpHeader {
    pub const SIZE: usize = 16;
}

impl LpHeader {
    pub fn new(receiver_idx: u32, counter: u64) -> Self {
        Self {
            protocol_version: version::CURRENT,
            reserved: [0u8; 3],
            receiver_idx,
            counter,
        }
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        // protocol version
        dst.put_u8(self.protocol_version);

        // reserved
        dst.put_slice(&self.reserved);

        // sender index
        dst.put_slice(&self.receiver_idx.to_le_bytes());

        // counter
        dst.put_slice(&self.counter.to_le_bytes());
    }

    pub fn parse(src: &[u8]) -> Result<Self, LpError> {
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

        let mut receiver_idx_bytes = [0u8; 4];
        receiver_idx_bytes.copy_from_slice(&src[4..8]);
        let receiver_idx = u32::from_le_bytes(receiver_idx_bytes);

        let mut counter_bytes = [0u8; 8];
        counter_bytes.copy_from_slice(&src[8..16]);
        let counter = u64::from_le_bytes(counter_bytes);

        Ok(LpHeader {
            protocol_version,
            reserved: [0u8; 3],
            receiver_idx,
            counter,
        })
    }

    /// Get the counter value from the header
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// Get the sender index from the header
    pub fn receiver_idx(&self) -> u32 {
        self.receiver_idx
    }
}

// subsequent data: MessageType || Data
