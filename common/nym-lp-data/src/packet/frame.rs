// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::packet::error::MalformedLpPacketError;
use bytes::{BufMut, Bytes, BytesMut};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Clone, PartialEq)]
pub struct LpFrameHeader {
    pub kind: LpFrameKind,
    pub frame_attributes: LpFrameAttributes,
}

impl LpFrameHeader {
    pub const SIZE: usize = 16; // message_kind(2) + message_attributes(14)

    pub fn new(kind: LpFrameKind, frame_attributes: LpFrameAttributes) -> Self {
        Self {
            kind,
            frame_attributes,
        }
    }

    pub fn new_no_attributes(kind: LpFrameKind) -> Self {
        Self {
            kind,
            frame_attributes: [0; 14],
        }
    }

    /// Encode directly into a BytesMut buffer
    pub fn encode(&self, dst: &mut BytesMut) {
        dst.put_u16_le(self.kind as u16);
        dst.put_slice(&self.frame_attributes);
    }

    pub fn parse(src: &[u8]) -> Result<Self, MalformedLpPacketError> {
        if src.len() < Self::SIZE {
            return Err(MalformedLpPacketError::InsufficientData);
        }
        let raw_kind = u16::from_le_bytes([src[0], src[1]]);

        let kind = LpFrameKind::try_from(raw_kind)
            .map_err(|_| MalformedLpPacketError::invalid_data_kind(raw_kind))?;

        #[allow(clippy::unwrap_used)]
        let message_attributes = src[2..16].try_into().unwrap();
        Ok(Self {
            kind,
            frame_attributes: message_attributes,
        })
    }
}

/// Represent application data being sent in Transport mode
#[derive(Debug, Clone, PartialEq)]
pub struct LpFrame {
    pub header: LpFrameHeader,
    pub content: Bytes,
}

impl AsRef<[u8]> for LpFrame {
    fn as_ref(&self) -> &[u8] {
        &self.content
    }
}

impl LpFrame {
    pub fn new(kind: LpFrameKind, content: impl Into<Bytes>) -> Self {
        Self {
            header: LpFrameHeader::new_no_attributes(kind),
            content: content.into(),
        }
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        self.header.encode(dst);

        dst.put_slice(&self.content);
    }

    pub fn decode(src: &[u8]) -> Result<Self, MalformedLpPacketError> {
        let header = LpFrameHeader::parse(src)?;
        let content = src[LpFrameHeader::SIZE..].to_vec().into();

        Ok(Self { header, content })
    }

    pub fn kind(&self) -> LpFrameKind {
        self.header.kind
    }

    pub fn new_opaque(content: impl Into<Bytes>) -> Self {
        Self::new(LpFrameKind::Opaque, content)
    }

    pub fn new_registration(data: impl Into<Bytes>) -> Self {
        Self::new(LpFrameKind::Registration, data)
    }

    pub fn new_forward(data: impl Into<Bytes>) -> Self {
        Self::new(LpFrameKind::Forward, data)
    }

    pub fn new_stream(attrs: SphinxStreamFrameAttributes, content: impl Into<Bytes>) -> Self {
        Self {
            header: LpFrameHeader::new(LpFrameKind::SphinxStream, attrs.encode()),
            content: content.into(),
        }
    }

    // is_empty in the sense len == 0 doesn't make sense in that case
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        LpFrameHeader::SIZE + self.content.len()
    }
}

/// Represent kind of application data being sent in Transport mode
#[derive(Clone, Copy, PartialEq, Eq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
pub enum LpFrameKind {
    Opaque = 0,
    Registration = 1,
    Forward = 2,
    SphinxStream = 3,
}

/// Message type within a `LpFrameKind::SphinxStream` frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SphinxStreamMsgType {
    /// Open a new stream. Content is optional initial data.
    Open = 0,
    /// Data on an existing stream.
    Data = 1,
}

/// Parsed form of the 14-byte `frame_attributes` for `LpFrameKind::SphinxStream`.
///
/// Wire layout (big-endian):
/// ```text
/// [0..8 ) stream_id    : u64
/// [8    ) msg_type      : u8   (0 = Open, 1 = Data)
/// [9..13) sequence_num  : u32
/// [13   ) reserved      : u8
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SphinxStreamFrameAttributes {
    pub stream_id: u64,
    pub msg_type: SphinxStreamMsgType,
    pub sequence_num: u32,
}

/// Raw 14-byte frame attributes field in every [`LpFrameHeader`].
/// Interpretation depends on the [`LpFrameKind`].
pub type LpFrameAttributes = [u8; 14];

impl SphinxStreamFrameAttributes {
    pub fn encode(&self) -> LpFrameAttributes {
        let mut buf = [0u8; 14];
        buf[0..8].copy_from_slice(&self.stream_id.to_be_bytes());
        buf[8] = self.msg_type as u8;
        buf[9..13].copy_from_slice(&self.sequence_num.to_be_bytes());
        buf
    }

    pub fn parse(attrs: &LpFrameAttributes) -> Result<Self, MalformedLpPacketError> {
        // SAFETY : 8 bytes slice into 8 bytes array
        #[allow(clippy::unwrap_used)]
        let stream_id = u64::from_be_bytes(attrs[0..8].try_into().unwrap());
        let msg_type = match attrs[8] {
            0 => SphinxStreamMsgType::Open,
            1 => SphinxStreamMsgType::Data,
            other => {
                return Err(MalformedLpPacketError::DeserialisationFailure(format!(
                    "invalid stream msg_type: {other}"
                )));
            }
        };
        // SAFETY : 4 bytes slice into 4 bytes array
        #[allow(clippy::unwrap_used)]
        let sequence_num = u32::from_be_bytes(attrs[9..13].try_into().unwrap());
        Ok(Self {
            stream_id,
            msg_type,
            sequence_num,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedResponseSize {
    /// We've sent a handshake message and expect response of predefined size
    Handshake(u32),

    /// We've sent a transport message and the response is length-prefixed
    Transport,
}

impl ExpectedResponseSize {
    pub fn to_bytes(&self) -> [u8; 4] {
        // there are no empty handshake messages, so we use 0 bytes to indicate Transport variant
        match self {
            ExpectedResponseSize::Handshake(size) => size.to_le_bytes(),
            ExpectedResponseSize::Transport => [0u8; 4],
        }
    }

    pub fn from_bytes(b: [u8; 4]) -> Self {
        let size = u32::from_le_bytes(b);
        if size == 0 {
            ExpectedResponseSize::Transport
        } else {
            ExpectedResponseSize::Handshake(size)
        }
    }
}

/// Packet forwarding request with embedded inner LP packet
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardPacketData {
    /// Target gateway's LP address (IP:port string)
    pub target_lp_address: SocketAddr,

    /// Indication of the expected size of the response
    /// to allow the proxy to read correct data from the stream
    pub expected_response_size: ExpectedResponseSize,

    /// Complete inner LP packet bytes (serialized LpPacket)
    /// This is the CLIENT→EXIT gateway packet, encrypted for exit
    pub inner_packet_bytes: Vec<u8>,
}

impl ForwardPacketData {
    pub fn new(
        target_lp_address: SocketAddr,
        expected_response_size: ExpectedResponseSize,
        inner_packet_bytes: Vec<u8>,
    ) -> Self {
        ForwardPacketData {
            target_lp_address,
            expected_response_size,
            inner_packet_bytes,
        }
    }

    // 0 || [4B ipv4]  || [2B port] || [4B res size] || [4B plen] || payload
    // 1 || [16B ipv6] || [2B port] || [4B res size] || [4B plen] || payload
    fn encode(&self, dst: &mut BytesMut) {
        let (is_ipv6, ip_bytes) = match &self.target_lp_address {
            SocketAddr::V4(address) => (false, address.ip().octets().to_vec()),
            SocketAddr::V6(address) => (true, address.ip().octets().to_vec()),
        };

        dst.put_u8(is_ipv6 as u8); // IP type , 0 for ipv4
        dst.put_slice(&ip_bytes); // IP bytes
        dst.put_u16_le(self.target_lp_address.port()); // Port
        dst.put_slice(&self.expected_response_size.to_bytes());
        dst.put_u32_le(self.inner_packet_bytes.len() as u32);
        dst.put_slice(&self.inner_packet_bytes);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();
        self.encode(&mut buf);
        buf.into()
    }

    pub fn decode(b: &[u8]) -> Result<Self, MalformedLpPacketError> {
        // smallest possible packet with ipv4 and empty data
        if b.len() < 15 {
            // 1 + 4 + 2 + 4 + 4 + 0
            return Err(MalformedLpPacketError::DeserialisationFailure(format!(
                "Too few bytes to deserialise ForwardPacketData. got {}",
                b.len()
            )));
        }

        let target_lp_address_is_ipv6 = b[0] != 0;

        let (target_lp_address, i) = if target_lp_address_is_ipv6 {
            // IPv6, first check we have actually enough bytes
            // smallest possible packet with ipv6 and empty data
            if b.len() < 27 {
                // 1 + 16 + 2 + 4 + 4+ 0
                return Err(MalformedLpPacketError::DeserialisationFailure(format!(
                    "Too few bytes to deserialise ipv6 ForwardPacketData. got {}",
                    b.len()
                )));
            }
            // Ipv6Addr::from_octets is not available until 1.91 so we have to use
            // the slightly less obvious u128 conversion
            // SAFETY: we ensured we have sufficient data, and the length is correct for casting
            #[allow(clippy::unwrap_used)]
            let ipv6 = IpAddr::V6(Ipv6Addr::from_bits(u128::from_be_bytes(
                b[1..17].try_into().unwrap(),
            )));
            let port = u16::from_le_bytes([b[17], b[18]]);
            (SocketAddr::new(ipv6, port), 19)
        } else {
            // IPv4. Length check done at the start

            // Ipv4Addr::from_octets is not available until 1.91
            let ipv4 = IpAddr::V4(Ipv4Addr::new(b[1], b[2], b[3], b[4]));
            let port = u16::from_le_bytes([b[5], b[6]]);
            (SocketAddr::new(ipv4, port), 7)
        };

        let expected_response_size_bytes = [b[i], b[i + 1], b[i + 2], b[i + 3]];
        let inner_packet_bytes_len = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]);

        if b[i + 8..].len() != inner_packet_bytes_len as usize {
            return Err(MalformedLpPacketError::DeserialisationFailure(format!(
                "Expected {inner_packet_bytes_len} bytes to deserialise inner packet bytes of ForwardPacketData. got {}",
                b[i + 8..].len()
            )));
        }
        let inner_packet_bytes = b[i + 8..].to_vec();

        Ok(ForwardPacketData {
            target_lp_address,
            expected_response_size: ExpectedResponseSize::from_bytes(expected_response_size_bytes),
            inner_packet_bytes,
        })
    }
}
