// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::packet::error::MalformedLpPacketError;
use bytes::{BufMut, Bytes, BytesMut};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Clone, PartialEq)]
pub struct LpMessageHeader {
    pub kind: LpMessageType,
    pub message_attributes: [u8; 14],
}

impl LpMessageHeader {
    pub const SIZE: usize = 16; // message_kind(2) + message_attributes(14)

    pub fn new(kind: LpMessageType, message_attributes: [u8; 14]) -> Self {
        Self {
            kind,
            message_attributes,
        }
    }

    pub fn new_no_attributes(kind: LpMessageType) -> Self {
        Self {
            kind,
            message_attributes: [0; 14],
        }
    }

    /// Encode directly into a BytesMut buffer
    pub fn encode(&self, dst: &mut BytesMut) {
        dst.put_u16_le(self.kind as u16);
        dst.put_slice(&self.message_attributes);
    }

    pub fn parse(src: &[u8]) -> Result<Self, MalformedLpPacketError> {
        if src.len() < Self::SIZE {
            return Err(MalformedLpPacketError::InsufficientData);
        }
        let raw_kind = u16::from_le_bytes([src[0], src[1]]);

        let kind = LpMessageType::try_from(raw_kind)
            .map_err(|_| MalformedLpPacketError::invalid_data_kind(raw_kind))?;

        #[allow(clippy::unwrap_used)]
        let message_attributes = src[2..16].try_into().unwrap();
        Ok(Self {
            kind,
            message_attributes,
        })
    }
}

/// Represent application data being sent in Transport mode
#[derive(Debug, Clone, PartialEq)]
pub struct LpMessage {
    pub header: LpMessageHeader,
    pub content: Bytes,
}

impl AsRef<[u8]> for LpMessage {
    fn as_ref(&self) -> &[u8] {
        &self.content
    }
}

impl LpMessage {
    pub fn new(kind: LpMessageType, content: impl Into<Bytes>) -> Self {
        Self {
            header: LpMessageHeader::new_no_attributes(kind),
            content: content.into(),
        }
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        self.header.encode(dst);

        dst.put_slice(&self.content);
    }

    pub fn decode(src: &[u8]) -> Result<Self, MalformedLpPacketError> {
        let header = LpMessageHeader::parse(src)?;
        let content = src[LpMessageHeader::SIZE..].to_vec().into();

        Ok(Self { header, content })
    }

    pub fn kind(&self) -> LpMessageType {
        self.header.kind
    }

    pub fn new_opaque(content: impl Into<Bytes>) -> Self {
        Self::new(LpMessageType::Opaque, content)
    }

    pub fn new_registration(data: impl Into<Bytes>) -> Self {
        Self::new(LpMessageType::Registration, data)
    }

    pub fn new_forward(data: impl Into<Bytes>) -> Self {
        Self::new(LpMessageType::Forward, data)
    }

    pub(crate) fn len(&self) -> usize {
        LpMessageHeader::SIZE + self.content.len()
    }
}

/// Represent kind of application data being sent in Transport mode
#[derive(Clone, Copy, PartialEq, Eq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
pub enum LpMessageType {
    Opaque = 0,
    Registration = 1,
    Forward = 2,
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
