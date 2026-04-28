// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Always available (light deps only)
pub mod error;
pub mod frame;
pub use error::MalformedLpPacketError;
pub use frame::{ForwardPacketData, LpFrame};

// Heavy: header uses peer_config (→ nym-crypto), replay uses LpError + LpPacket
#[cfg(feature = "full")]
pub mod header;
#[cfg(feature = "full")]
pub mod replay;

#[cfg(feature = "full")]
pub use header::{InnerHeader, LpHeader, OuterHeader};

pub mod version {
    /// The current version of the Lewes Protocol that is put into each new constructed header.
    pub const CURRENT: u8 = 1;
}

#[allow(dead_code)]
pub(crate) const UDP_HEADER_LEN: usize = 8;
#[allow(dead_code)]
pub(crate) const IP_HEADER_LEN: usize = 40; // v4 - 20, v6 - 40
#[allow(dead_code)]
pub(crate) const MTU: usize = 1500;
#[allow(dead_code)]
pub(crate) const UDP_OVERHEAD: usize = UDP_HEADER_LEN + IP_HEADER_LEN;
#[allow(dead_code)]
pub(crate) const UDP_PAYLOAD_SIZE: usize = MTU - UDP_OVERHEAD;

#[cfg(feature = "full")]
use bytes::{BufMut, BytesMut};
#[cfg(feature = "full")]
use nym_common::debug::format_debug_bytes;
#[cfg(feature = "full")]
use std::fmt::{Debug, Formatter};

#[cfg(feature = "full")]
#[derive(Clone)]
pub struct EncryptedLpPacket {
    // The outer header that's sent in plaintext
    pub(crate) outer_header: OuterHeader,

    // The ciphertext containing the inner header and the payload
    pub(crate) ciphertext: Vec<u8>,
}

#[cfg(feature = "full")]
impl std::fmt::Debug for EncryptedLpPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_debug_bytes(&self.debug_bytes())?)
    }
}

#[cfg(feature = "full")]
impl EncryptedLpPacket {
    pub fn new(outer_header: OuterHeader, ciphertext: Vec<u8>) -> EncryptedLpPacket {
        EncryptedLpPacket {
            outer_header,
            ciphertext,
        }
    }

    pub fn encoded_length(&self) -> usize {
        OuterHeader::SIZE + self.ciphertext.len()
    }

    pub(crate) fn debug_bytes(&self) -> Vec<u8> {
        let mut bytes = BytesMut::new();
        self.encode(&mut bytes);
        bytes.freeze().to_vec()
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        self.outer_header.encode(dst);
        dst.put_slice(&self.ciphertext)
    }

    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    pub fn outer_header(&self) -> OuterHeader {
        self.outer_header
    }
}

#[cfg(feature = "full")]
#[derive(Clone, PartialEq)]
pub struct LpPacket {
    pub(crate) header: LpHeader,
    pub(crate) frame: LpFrame,
}

#[cfg(feature = "full")]
impl Debug for LpPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_debug_bytes(&self.debug_bytes())?)
    }
}

#[cfg(feature = "full")]
impl LpPacket {
    pub fn new(header: LpHeader, frame: LpFrame) -> Self {
        Self { header, frame }
    }

    pub fn frame(&self) -> &LpFrame {
        &self.frame
    }

    pub fn into_frame(self) -> LpFrame {
        self.frame
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
        self.frame.encode(dst)
    }
}
