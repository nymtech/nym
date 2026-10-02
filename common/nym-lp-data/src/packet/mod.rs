// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::{BufMut, BytesMut};
use std::fmt::{Debug, Formatter};

use nym_common::debug::format_debug_bytes;

pub use error::MalformedLpPacketError;
pub use frame::{ForwardPacketData, LpFrame};
pub use header::{InnerHeader, LpHeader, OuterHeader};

pub mod error;
pub mod frame;
pub mod header;
pub mod version;

#[allow(dead_code)]
pub(crate) const UDP_HEADER_LEN: usize = 8;
#[allow(dead_code)]
pub(crate) const IP_HEADER_LEN: usize = 40; // v4 - 20, v6 - 40
pub const MTU: usize = 1500;
#[allow(dead_code)]
pub(crate) const UDP_OVERHEAD: usize = UDP_HEADER_LEN + IP_HEADER_LEN;
#[allow(dead_code)]
pub(crate) const UDP_PAYLOAD_SIZE: usize = MTU - UDP_OVERHEAD;

/// What the AEAD encrytpion adds
/// 8 bytes channel id, 16 bytes AEAD tag, 2 bytes TLS length serialization (for the lenght we are targetting)
pub(crate) const AEAD_ENCRYPTION_OVERHEAD: usize = 26;

/// An LP packet as it appears on the wire: `OuterHeader` in the clear, everything else encrypted.
///
/// The plaintext counterpart is [`LpPacket`]. Note the two do not have the same size: encrypting
/// expands the payload, so budget with [`Self::OVERHEAD`] and not [`LpHeader::SIZE`].
#[derive(Clone)]
pub struct EncryptedLpPacket {
    // The outer header that's sent in plaintext
    pub(crate) outer_header: OuterHeader,

    // The ciphertext containing the inner header and the payload
    pub(crate) ciphertext: Vec<u8>,
}

impl std::fmt::Debug for EncryptedLpPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_debug_bytes(&self.debug_bytes())?)
    }
}

impl EncryptedLpPacket {
    /// Bytes an encrypted packet adds over the frame it carries: the cleartext outer header, the
    /// encrypted inner header, and the AEAD expansion.
    pub const OVERHEAD: usize = LpHeader::SIZE + AEAD_ENCRYPTION_OVERHEAD;

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

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();
        self.encode(&mut buf);
        buf.to_vec()
    }

    pub fn decode(src: &[u8]) -> Result<Self, MalformedLpPacketError> {
        let outer_header = OuterHeader::parse(src)?;
        let ciphertext = src[OuterHeader::SIZE..].to_vec();

        Ok(Self {
            outer_header,
            ciphertext,
        })
    }

    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    pub fn outer_header(&self) -> OuterHeader {
        self.outer_header
    }
}

/// An LP packet in the clear: the full [`LpHeader`] plus its frame, never sent as-is.
///
/// This is what a sender builds and a receiver gets back after decryption; the wire form is
/// [`EncryptedLpPacket`]. Costs [`LpHeader::SIZE`] over its frame, which is *less* than the
/// encrypted form costs - so it is the wrong figure to size a frame against.
#[derive(Clone, PartialEq)]
pub struct LpPacket {
    pub(crate) header: LpHeader,
    pub(crate) frame: LpFrame,
}

impl Debug for LpPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_debug_bytes(&self.debug_bytes())?)
    }
}

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

    // SW TMP, while we don't have any encryption
    pub fn decode(packet: EncryptedLpPacket) -> Result<Self, MalformedLpPacketError> {
        let plaintext = packet.ciphertext();
        let inner_header = InnerHeader::parse(plaintext)?;
        let payload = &plaintext[InnerHeader::SIZE..];
        let frame = LpFrame::decode(payload)?;

        Ok(Self::new(
            LpHeader {
                outer: packet.outer_header(),
                inner: inner_header,
            },
            frame,
        ))
    }

    // SW TMP, while we don't have any encryption
    pub fn encode(self) -> EncryptedLpPacket {
        // Outer header gets serialized by EncryptedLpPacket so we need to not serialize it as part of LpPacket
        let outer_header = self.header.outer;

        // LpPacket bytes without outerheader
        let mut bytes = BytesMut::new();
        self.header.inner.encode(&mut bytes);
        self.frame.encode(&mut bytes);
        let ciphertext = bytes.freeze().to_vec();

        EncryptedLpPacket::new(outer_header, ciphertext)
    }
}
