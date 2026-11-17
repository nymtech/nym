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

pub const UDP_HEADER_LEN: usize = 8;

// v4 - 20, v6 - 40
pub const IP_HEADER_LEN: usize = 40;

/// The path MTU LP datagrams are sized for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathMtu {
    /// Ethernet.
    Standard = 1500,

    /// The IPv6 minimum: survives tunnels and PPPoE, at the cost of smaller packets.
    Ipv6Minimum = 1280,
}

impl PathMtu {
    pub const fn mtu(self) -> usize {
        self as usize
    }

    /// One UDP datagram's payload once the IP and UDP headers have taken their cut.
    pub const fn max_udp_payload_size(self) -> usize {
        self.mtu() - IP_HEADER_LEN - UDP_HEADER_LEN
    }

    /// One frame, header included, once the encryption layer has also taken its cut.
    pub const fn max_frame_size(self) -> usize {
        self.max_udp_payload_size() - EncryptedLpPacket::OVERHEAD
    }

    /// What is left of one frame for its payload.
    pub const fn max_frame_payload_size(self) -> usize {
        self.max_frame_size() - frame::LpFrameHeader::SIZE
    }
}

/// The path every LP size below is derived from.
pub const PATH_MTU: PathMtu = PathMtu::Standard;

/// The datagram budget: what a transport may put into one UDP packet.
pub const MAX_UDP_PAYLOAD_SIZE: usize = PATH_MTU.max_udp_payload_size();

/// The frame budget, header included: what a node may hand to the transport wrap.
pub const MAX_FRAME_SIZE: usize = PATH_MTU.max_frame_size();

/// What is left for a frame's payload in one datagram, once both wire layers have taken their cut.
///
/// This is the budget a message is chunked against, and chunk size decides where a message is
/// split - which is visible in the size of the packets that come out. So it is not only the wire
/// limit for something that meets a wire, but the size *any* pipeline emitting LP-shaped packets
/// has to match, including one whose own frames never leave the process.
pub const MAX_FRAME_PAYLOAD_SIZE: usize = PATH_MTU.max_frame_payload_size();

/// How many frames one sphinx packet spans.
///
/// A sphinx packet is around 2KB and [`MAX_FRAME_PAYLOAD_SIZE`] is well under that, so it is always
/// split - and always into this many. Stated rather than computed because the sphinx packet's size
/// belongs to a layer this crate does not depend on; the two are related only through the wrapping
/// pipeline, which multiplies its frame budget by this to arrive at a chunk size.
pub const FRAMES_PER_SPHINX_PACKET: usize = 2;

/// What the AEAD encryption adds
/// 8 bytes channel id, 16 bytes AEAD tag, 2 bytes TLS length serialization (for the length we are targeting)
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
}
