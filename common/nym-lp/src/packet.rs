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

// VERSION [1B] || RESERVED [3B] || SENDER_INDEX [4B] || COUNTER [8B]
#[derive(Debug, Clone)]
pub struct LpHeader {
    pub protocol_version: u8,
    pub reserved: u16,
    pub session_id: u32,
    pub counter: u64,
}

impl LpHeader {
    pub const SIZE: usize = 16;
}

impl LpHeader {
    pub fn new(session_id: u32, counter: u64) -> Self {
        Self {
            protocol_version: 1,
            reserved: 0,
            session_id,
            counter,
        }
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        // protocol version
        dst.put_u8(self.protocol_version);

        // reserved
        dst.put_slice(&[0, 0, 0]);

        // sender index
        dst.put_slice(&self.session_id.to_le_bytes());

        // counter
        dst.put_slice(&self.counter.to_le_bytes());
    }

    pub fn parse(src: &[u8]) -> Result<Self, LpError> {
        if src.len() < Self::SIZE {
            return Err(LpError::InsufficientBufferSize);
        }

        let protocol_version = src[0];
        // Skip reserved bytes [1..4]

        let mut session_id_bytes = [0u8; 4];
        session_id_bytes.copy_from_slice(&src[4..8]);
        let session_id = u32::from_le_bytes(session_id_bytes);

        let mut counter_bytes = [0u8; 8];
        counter_bytes.copy_from_slice(&src[8..16]);
        let counter = u64::from_le_bytes(counter_bytes);

        Ok(LpHeader {
            protocol_version,
            reserved: 0,
            session_id,
            counter,
        })
    }

    /// Get the counter value from the header
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// Get the sender index from the header
    pub fn session_id(&self) -> u32 {
        self.session_id
    }
}

// subsequent data: MessageType || Data
