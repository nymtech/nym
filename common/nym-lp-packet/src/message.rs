// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MalformedLpPacketError;
use bytes::{BufMut, BytesMut};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use nym_crypto::asymmetric::ed25519;
use std::fmt;
use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Copy, Clone, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum MessageType {
    /// The party is busy
    Busy = 0x0000,

    /// Encrypted payload
    EncryptedData = 0x0001,

    /// Receiver should forward this message via telescoping
    ForwardPacket = 0x0002,

    /// Receiver index collision - client should retry with new index
    Collision = 0x0003,

    /// Acknowledgment - gateway confirms receipt of message
    Ack = 0x0004,

    /// General error
    Error = 0x00FF,
}

impl MessageType {
    pub(crate) fn from_u32(value: u32) -> Option<Self> {
        MessageType::try_from(value).ok()
    }

    pub fn to_u32(&self) -> u32 {
        u32::from(*self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationData(pub Vec<u8>);

impl ApplicationData {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.0);
    }

    fn decode(bytes: &[u8]) -> Result<Self, MalformedLpPacketError> {
        Ok(ApplicationData(bytes.to_vec()))
    }
}

/// General human-readable error message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorPacketData {
    pub message: String,
}

impl ErrorPacketData {
    pub fn new(message: impl Into<String>) -> Self {
        ErrorPacketData {
            message: message.into(),
        }
    }

    fn len(&self) -> usize {
        // length-encoding + message
        4 + self.message.len()
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_u32_le(self.message.len() as u32);
        dst.put_slice(self.message.as_bytes());
    }

    fn decode(bytes: &[u8]) -> Result<Self, MalformedLpPacketError> {
        if bytes.len() < 4 {
            return Err(MalformedLpPacketError::DeserialisationFailure(format!(
                "Too few bytes to deserialise ErrorPacketData. got {}",
                bytes.len()
            )));
        }

        let message_len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if bytes[4..].len() != message_len {
            return Err(MalformedLpPacketError::DeserialisationFailure(format!(
                "Wrong number of bytes to deserialise ErrorPacketData. got {}. Expected {}",
                bytes.len(),
                4 + message_len
            )));
        }

        let message = String::from_utf8_lossy(&bytes[4..]).to_string();

        Ok(ErrorPacketData { message })
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

    fn len(&self) -> usize {
        // 1 byte length of target lp address type
        // +
        // {4,16} target_lp_address IPv{4,6}
        // +
        // 2 bytes target_lp_address port
        // +
        // 4 bytes for expected response size
        // +
        // 4 bytes of length of inner packet bytes
        // +
        // inner_packet_bytes.len()
        match self.target_lp_address {
            SocketAddr::V4(_) => 1 + 4 + 2 + 4 + 4 + self.inner_packet_bytes.len(),
            SocketAddr::V6(_) => 1 + 16 + 2 + 4 + 4 + self.inner_packet_bytes.len(),
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
            // SAFETY: we ensured we have sufficient data, and the length is correct for casting
            #[allow(clippy::unwrap_used)]
            let ipv6 = IpAddr::V6(Ipv6Addr::from_octets(b[1..17].try_into().unwrap()));
            let port = u16::from_le_bytes([b[17], b[18]]);
            (SocketAddr::new(ipv6, port), 19)
        } else {
            // IPv4. Length check done at the start
            // SAFETY: we ensured we have sufficient data, and the length is correct for casting
            #[allow(clippy::unwrap_used)]
            let ipv4 = IpAddr::V4(Ipv4Addr::from_octets(b[1..5].try_into().unwrap()));
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

#[derive(Debug, Clone)]
pub enum LpMessage {
    /// The party is busy
    Busy,

    /// Application payload is being sent
    ApplicationData(ApplicationData),

    /// Receiver should forward this message via telescoping
    ForwardPacket(ForwardPacketData),

    /// Receiver index collision - client should retry with new receiver_index
    Collision,

    /// Acknowledgment - gateway confirms receipt of message
    Ack,

    /// An error has occurred
    Error(ErrorPacketData),
}

impl From<ApplicationData> for LpMessage {
    fn from(value: ApplicationData) -> Self {
        LpMessage::ApplicationData(value)
    }
}

impl From<ForwardPacketData> for LpMessage {
    fn from(value: ForwardPacketData) -> Self {
        LpMessage::ForwardPacket(value)
    }
}

impl Display for LpMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LpMessage::Busy => write!(f, "Busy"),
            LpMessage::ApplicationData(_) => write!(f, "EncryptedData"),
            LpMessage::ForwardPacket(_) => write!(f, "ForwardPacket"),
            LpMessage::Collision => write!(f, "Collision"),
            LpMessage::Ack => write!(f, "Ack"),
            LpMessage::Error(_) => write!(f, "Error"),
        }
    }
}

impl LpMessage {
    #[deprecated(note = "is it actually needed?")]
    pub fn payload(&self) -> &[u8] {
        match self {
            LpMessage::Busy => &[],
            LpMessage::ApplicationData(payload) => payload.0.as_slice(),
            LpMessage::ForwardPacket(_) => &[], // Structured data, serialized in encode_content
            LpMessage::Collision => &[],
            LpMessage::Ack => &[],
            LpMessage::Error(_) => &[], // Structured data, serialized in encode_content (?)
        }
    }

    #[deprecated(note = "is it actually needed?")]
    pub fn is_empty(&self) -> bool {
        match self {
            LpMessage::Busy => true,
            LpMessage::ApplicationData(payload) => payload.0.is_empty(),
            LpMessage::ForwardPacket(_) => false, // Always has data
            LpMessage::Collision => true,
            LpMessage::Ack => true,
            LpMessage::Error(_) => false,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            LpMessage::Busy => 0,
            LpMessage::ApplicationData(payload) => payload.len(),
            LpMessage::ForwardPacket(payload) => payload.len(),
            LpMessage::Collision => 0,
            LpMessage::Ack => 0,
            LpMessage::Error(payload) => payload.len(),
        }
    }

    pub fn typ(&self) -> MessageType {
        match self {
            LpMessage::Busy => MessageType::Busy,
            LpMessage::ApplicationData(_) => MessageType::EncryptedData,
            LpMessage::ForwardPacket(_) => MessageType::ForwardPacket,
            LpMessage::Collision => MessageType::Collision,
            LpMessage::Ack => MessageType::Ack,
            LpMessage::Error(_) => MessageType::Error,
        }
    }

    pub fn encode_content(&self, dst: &mut BytesMut) {
        match self {
            LpMessage::Busy => { /* No content */ }
            LpMessage::ApplicationData(payload) => payload.encode(dst),
            LpMessage::ForwardPacket(data) => data.encode(dst),
            LpMessage::Collision => { /* No content */ }
            LpMessage::Ack => { /* No content */ }
            LpMessage::Error(data) => data.encode(dst),
        }
    }

    /// Parse message from its type and content bytes.
    ///
    /// Used when decrypting outer-encrypted packets where the message type
    /// was encrypted along with the content.
    pub fn decode_content(
        content: &[u8],
        message_type: MessageType,
    ) -> Result<Self, MalformedLpPacketError> {
        match message_type {
            MessageType::Busy => {
                content.ensure_empty()?;
                Ok(LpMessage::Busy)
            }
            MessageType::EncryptedData => Ok(LpMessage::ApplicationData(ApplicationData::decode(
                content,
            )?)),
            MessageType::ForwardPacket => Ok(LpMessage::ForwardPacket(ForwardPacketData::decode(
                content,
            )?)),
            MessageType::Collision => {
                content.ensure_empty()?;
                Ok(LpMessage::Collision)
            }
            MessageType::Ack => {
                content.ensure_empty()?;
                Ok(LpMessage::Ack)
            }
            MessageType::Error => Ok(LpMessage::Error(ErrorPacketData::decode(content)?)),
        }
    }
}

/// Helper trait for improving readability to return error if bytes content is not empty
trait EnsureEmptyContent {
    fn ensure_empty(&self) -> Result<(), MalformedLpPacketError>;
}

impl EnsureEmptyContent for &[u8] {
    fn ensure_empty(&self) -> Result<(), MalformedLpPacketError> {
        if !self.is_empty() {
            return Err(MalformedLpPacketError::InvalidPayloadSize {
                expected: 0,
                actual: self.len(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InnerHeader, LpHeader, LpPacket, OuterHeader};

    #[test]
    fn encoding() {
        let message = LpMessage::ApplicationData(ApplicationData(vec![11u8; 124]));

        let resp_header = LpHeader {
            outer: OuterHeader {
                receiver_idx: 456,
                counter: 123,
            },
            inner: InnerHeader {
                protocol_version: 1,
                reserved: [0u8; 3],
                message_type: MessageType::EncryptedData,
            },
        };

        let packet = LpPacket {
            header: resp_header,
            message,
        };

        // Just print packet for debug, will be captured in test output
        println!("{packet:?}");

        // Verify message type
        assert!(matches!(packet.message.typ(), MessageType::EncryptedData));

        // Verify correct data in message
        match &packet.message {
            LpMessage::ApplicationData(data) => {
                assert_eq!(*data, ApplicationData(vec![11u8; 124]));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn forward_message_encoding() {
        let msg1 = ForwardPacketData {
            target_lp_address: "1.2.3.4:5678".parse().unwrap(),
            expected_response_size: ExpectedResponseSize::Transport,
            inner_packet_bytes: vec![],
        };

        let msg2 = ForwardPacketData {
            target_lp_address: "1.2.3.4:5678".parse().unwrap(),
            expected_response_size: ExpectedResponseSize::Handshake(250),
            inner_packet_bytes: vec![42u8; 64],
        };

        let msg3 = ForwardPacketData {
            target_lp_address: "[2001:db8::1]:8080".parse().unwrap(),
            expected_response_size: ExpectedResponseSize::Transport,
            inner_packet_bytes: vec![],
        };

        let msg4 = ForwardPacketData {
            target_lp_address: "[2001:db8::1]:8080".parse().unwrap(),
            expected_response_size: ExpectedResponseSize::Handshake(250),
            inner_packet_bytes: vec![42u8; 64],
        };

        let b = msg1.to_bytes();
        let msg1_r = ForwardPacketData::decode(&b).unwrap();
        assert_eq!(msg1_r, msg1);

        let b = msg2.to_bytes();
        let msg2_r = ForwardPacketData::decode(&b).unwrap();
        assert_eq!(msg2_r, msg2);

        let b = msg3.to_bytes();
        let msg3_r = ForwardPacketData::decode(&b).unwrap();
        assert_eq!(msg3_r, msg3);

        let b = msg4.to_bytes();
        let msg4_r = ForwardPacketData::decode(&b).unwrap();
        assert_eq!(msg4_r, msg4);
    }
}
