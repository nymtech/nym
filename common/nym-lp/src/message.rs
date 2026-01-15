// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{BOOTSTRAP_RECEIVER_IDX, LpError};
use bytes::{BufMut, BytesMut};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Data structure for the ClientHello message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHelloData {
    /// Client-proposed receiver index for session identification (4 bytes)
    /// Auto-generated randomly by the client
    pub receiver_index: u32,
    /// Client's LP x25519 public key (32 bytes) - derived from Ed25519 key
    pub client_lp_public_key: [u8; 32],
    /// Client's Ed25519 public key (32 bytes) - for PSQ authentication
    pub client_ed25519_public_key: [u8; 32],
    /// Salt for PSK derivation (32 bytes: 8-byte timestamp + 24-byte nonce)
    pub salt: [u8; 32],
}

impl ClientHelloData {
    // 4 bytes for receiver index + 32 bytes for client lp key, 32 bytes for client ed25519 key + 32 bytes for salt
    pub const LEN: usize = 100;

    fn len(&self) -> usize {
        Self::LEN
    }

    fn generate_receiver_index() -> u32 {
        loop {
            let candidate = rand::random();
            if candidate != BOOTSTRAP_RECEIVER_IDX {
                return candidate;
            }
        }
    }

    /// Generates a new ClientHelloData with fresh salt.
    ///
    /// Salt format: 8 bytes timestamp (u64 LE) + 24 bytes random nonce
    ///
    /// # Arguments
    /// * `client_lp_public_key` - Client's x25519 public key (derived from Ed25519)
    /// * `client_ed25519_public_key` - Client's Ed25519 public key (for PSQ authentication)
    pub fn new_with_fresh_salt(
        client_lp_public_key: [u8; 32],
        client_ed25519_public_key: [u8; 32],
        timestamp: u64,
    ) -> Self {
        // Generate salt: timestamp + nonce
        let mut salt = [0u8; 32];

        // First 8 bytes: current timestamp as u64 little-endian
        salt[..8].copy_from_slice(&timestamp.to_le_bytes());

        // Last 24 bytes: random nonce
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut salt[8..]);

        Self {
            receiver_index: Self::generate_receiver_index(), // Auto-generate random receiver index
            client_lp_public_key,
            client_ed25519_public_key,
            salt,
        }
    }

    /// Extracts the timestamp from the salt.
    ///
    /// # Returns
    /// Unix timestamp in seconds
    pub fn extract_timestamp(&self) -> u64 {
        let mut timestamp_bytes = [0u8; 8];
        timestamp_bytes.copy_from_slice(&self.salt[..8]);
        u64::from_le_bytes(timestamp_bytes)
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        dst.put_u32_le(self.receiver_index);
        dst.put_slice(&self.client_lp_public_key);
        dst.put_slice(&self.client_ed25519_public_key);
        dst.put_slice(&self.salt);
    }

    pub fn decode(b: &[u8]) -> Result<Self, LpError> {
        if b.len() != Self::LEN {
            return Err(LpError::DeserializationError(format!(
                "Expected {} bytes to deserialise ClientHelloData. got {}",
                Self::LEN,
                b.len()
            )));
        }

        // SAFETY: we checked for valid byte lengths
        #[allow(clippy::unwrap_used)]
        Ok(ClientHelloData {
            receiver_index: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            client_lp_public_key: b[4..36].try_into().unwrap(),
            client_ed25519_public_key: b[36..68].try_into().unwrap(),
            salt: b[68..].try_into().unwrap(),
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum MessageType {
    Busy = 0x0000,
    Handshake = 0x0001,
    EncryptedData = 0x0002,
    ClientHello = 0x0003,
    KKTRequest = 0x0004,
    KKTResponse = 0x0005,
    ForwardPacket = 0x0006,
    /// Receiver index collision - client should retry with new index
    Collision = 0x0007,
    /// Acknowledgment - gateway confirms receipt of message
    Ack = 0x0008,
    /// Subsession request - client initiates subsession creation
    SubsessionRequest = 0x0009,
    /// Subsession KK1 - first message of Noise KK handshake
    SubsessionKK1 = 0x000A,
    /// Subsession KK2 - second message of Noise KK handshake
    SubsessionKK2 = 0x000B,
    /// Subsession ready - subsession established confirmation
    SubsessionReady = 0x000C,
    /// Subsession abort - race winner tells loser to become responder
    SubsessionAbort = 0x000D,
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
pub struct HandshakeData(pub Vec<u8>);

impl HandshakeData {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.0);
    }

    fn decode(bytes: &[u8]) -> Result<Self, LpError> {
        Ok(HandshakeData(bytes.to_vec()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedDataPayload(pub Vec<u8>);

impl EncryptedDataPayload {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.0);
    }

    fn decode(bytes: &[u8]) -> Result<Self, LpError> {
        Ok(EncryptedDataPayload(bytes.to_vec()))
    }
}

/// KKT request frame data (serialized KKTFrame bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KKTRequestData(pub Vec<u8>);

impl KKTRequestData {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.0);
    }

    fn decode(bytes: &[u8]) -> Result<Self, LpError> {
        Ok(KKTRequestData(bytes.to_vec()))
    }
}

/// KKT response frame data (serialized KKTFrame bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KKTResponseData(pub Vec<u8>);

impl KKTResponseData {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.0);
    }

    fn decode(bytes: &[u8]) -> Result<Self, LpError> {
        Ok(KKTResponseData(bytes.to_vec()))
    }
}

/// Packet forwarding request with embedded inner LP packet
#[derive(Debug, Clone)]
pub struct ForwardPacketData {
    /// Target gateway's Ed25519 identity (32 bytes)
    pub target_gateway_identity: [u8; 32],

    // TODO: replace it with `SocketAddr`
    /// Target gateway's LP address (IP:port string)
    pub target_lp_address: String,

    /// Complete inner LP packet bytes (serialized LpPacket)
    /// This is the CLIENT→EXIT gateway packet, encrypted for exit
    pub inner_packet_bytes: Vec<u8>,
}

impl ForwardPacketData {
    fn len(&self) -> usize {
        // 32 bytes target gateway identity
        // +
        // 4 bytes length of target lp address
        // +
        // target_lp_address.len()
        // +
        // 4 bytes of length of inner packet bytes
        // +
        // inner_packet_bytes.len()
        32 + 4 + self.target_lp_address.len() + 4 + self.inner_packet_bytes.len()
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.target_gateway_identity);
        dst.put_u16_le(self.target_lp_address.len() as u16);
        dst.put_slice(self.target_lp_address.as_bytes());
        dst.put_u32_le(self.inner_packet_bytes.len() as u32);
        dst.put_slice(&self.inner_packet_bytes);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();
        self.encode(&mut buf);
        buf.into()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, LpError> {
        // smallest possible packet with empty address and empty data
        if bytes.len() < 38 {
            return Err(LpError::DeserializationError(format!(
                "Too few bytes to deserialise ForwardPacketData[1]. got {}",
                bytes.len()
            )));
        }
        // SAFETY: we ensured we have sufficient data
        #[allow(clippy::unwrap_used)]
        let target_gateway_identity = bytes[0..32].try_into().unwrap();
        let target_lp_address_len = u16::from_le_bytes([bytes[32], bytes[33]]);

        // smallest possible packet with empty data
        if bytes[34..].len() < 4 + target_lp_address_len as usize {
            return Err(LpError::DeserializationError(format!(
                "Too few bytes to deserialise ForwardPacketData[2]. got {}",
                bytes.len()
            )));
        }

        let target_lp_address =
            String::from_utf8_lossy(&bytes[34..34 + target_lp_address_len as usize]).to_string();
        let inner_packet_bytes_len = u32::from_le_bytes([
            bytes[34 + target_lp_address_len as usize],
            bytes[34 + target_lp_address_len as usize + 1],
            bytes[34 + target_lp_address_len as usize + 2],
            bytes[34 + target_lp_address_len as usize + 3],
        ]);
        if bytes[34 + target_lp_address_len as usize + 4..].len() != inner_packet_bytes_len as usize
        {
            return Err(LpError::DeserializationError(format!(
                "Expected {inner_packet_bytes_len} bytes to deserialise inner packet bytes of ForwardPacketData. got {}",
                bytes[34 + target_lp_address_len as usize + 4..].len()
            )));
        }
        let inner_packet_bytes = bytes[34 + target_lp_address_len as usize + 4..].to_vec();

        Ok(ForwardPacketData {
            target_gateway_identity,
            target_lp_address,
            inner_packet_bytes,
        })
    }
}

/// Subsession KK1 message - first message of Noise KK handshake
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubsessionKK1Data {
    /// Noise KK first message payload (ephemeral key + encrypted static)
    pub payload: Vec<u8>,
}

impl SubsessionKK1Data {
    fn len(&self) -> usize {
        self.payload.len()
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.payload);
    }

    fn decode(bytes: &[u8]) -> Result<Self, LpError> {
        Ok(SubsessionKK1Data {
            payload: bytes.to_vec(),
        })
    }
}

/// Subsession KK2 message - second message of Noise KK handshake
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubsessionKK2Data {
    /// Noise KK second message payload (ephemeral key + encrypted response)
    pub payload: Vec<u8>,
}

impl SubsessionKK2Data {
    fn len(&self) -> usize {
        self.payload.len()
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.payload);
    }

    fn decode(bytes: &[u8]) -> Result<Self, LpError> {
        Ok(SubsessionKK2Data {
            payload: bytes.to_vec(),
        })
    }
}

/// Subsession ready confirmation with new session index
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubsessionReadyData {
    /// New subsession's receiver index for routing
    pub receiver_index: u32,
}

impl SubsessionReadyData {
    pub const LEN: usize = 4;

    fn len(&self) -> usize {
        Self::LEN
    }

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_u32_le(self.receiver_index);
    }

    fn decode(bytes: &[u8]) -> Result<Self, LpError> {
        if bytes.len() != 4 {
            return Err(LpError::DeserializationError(format!(
                "Expected 4 bytes to deserialise SubsessionReadyData. got {}",
                bytes.len()
            )));
        }
        Ok(SubsessionReadyData {
            receiver_index: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }
}

#[derive(Debug, Clone)]
pub enum LpMessage {
    Busy,
    Handshake(HandshakeData),
    EncryptedData(EncryptedDataPayload),
    ClientHello(ClientHelloData),
    KKTRequest(KKTRequestData),
    KKTResponse(KKTResponseData),
    ForwardPacket(ForwardPacketData),
    /// Receiver index collision - client should retry with new receiver_index
    Collision,
    /// Acknowledgment - gateway confirms receipt of message
    Ack,
    /// Subsession request - client initiates subsession creation (empty, signal only)
    SubsessionRequest,
    /// Subsession KK1 - first message of Noise KK handshake
    SubsessionKK1(SubsessionKK1Data),
    /// Subsession KK2 - second message of Noise KK handshake
    SubsessionKK2(SubsessionKK2Data),
    /// Subsession ready - subsession established confirmation
    SubsessionReady(SubsessionReadyData),
    /// Subsession abort - race winner tells loser to become responder (empty, signal only)
    SubsessionAbort,
}

impl Display for LpMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LpMessage::Busy => write!(f, "Busy"),
            LpMessage::Handshake(_) => write!(f, "Handshake"),
            LpMessage::EncryptedData(_) => write!(f, "EncryptedData"),
            LpMessage::ClientHello(_) => write!(f, "ClientHello"),
            LpMessage::KKTRequest(_) => write!(f, "KKTRequest"),
            LpMessage::KKTResponse(_) => write!(f, "KKTResponse"),
            LpMessage::ForwardPacket(_) => write!(f, "ForwardPacket"),
            LpMessage::Collision => write!(f, "Collision"),
            LpMessage::Ack => write!(f, "Ack"),
            LpMessage::SubsessionRequest => write!(f, "SubsessionRequest"),
            LpMessage::SubsessionKK1(_) => write!(f, "SubsessionKK1"),
            LpMessage::SubsessionKK2(_) => write!(f, "SubsessionKK2"),
            LpMessage::SubsessionReady(_) => write!(f, "SubsessionReady"),
            LpMessage::SubsessionAbort => write!(f, "SubsessionAbort"),
        }
    }
}

impl LpMessage {
    pub fn payload(&self) -> &[u8] {
        match self {
            LpMessage::Busy => &[],
            LpMessage::Handshake(payload) => payload.0.as_slice(),
            LpMessage::EncryptedData(payload) => payload.0.as_slice(),
            LpMessage::ClientHello(_) => &[], // Structured data, serialized in encode_content
            LpMessage::KKTRequest(payload) => payload.0.as_slice(),
            LpMessage::KKTResponse(payload) => payload.0.as_slice(),
            LpMessage::ForwardPacket(_) => &[], // Structured data, serialized in encode_content
            LpMessage::Collision => &[],
            LpMessage::Ack => &[],
            LpMessage::SubsessionRequest => &[],
            LpMessage::SubsessionKK1(_) => &[], // Structured data, serialized in encode_content
            LpMessage::SubsessionKK2(_) => &[], // Structured data, serialized in encode_content
            LpMessage::SubsessionReady(_) => &[], // Structured data, serialized in encode_content
            LpMessage::SubsessionAbort => &[],
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            LpMessage::Busy => true,
            LpMessage::Handshake(payload) => payload.0.is_empty(),
            LpMessage::EncryptedData(payload) => payload.0.is_empty(),
            LpMessage::ClientHello(_) => false, // Always has data
            LpMessage::KKTRequest(payload) => payload.0.is_empty(),
            LpMessage::KKTResponse(payload) => payload.0.is_empty(),
            LpMessage::ForwardPacket(_) => false, // Always has data
            LpMessage::Collision => true,
            LpMessage::Ack => true,
            LpMessage::SubsessionRequest => true, // Empty signal
            LpMessage::SubsessionKK1(_) => false, // Always has payload
            LpMessage::SubsessionKK2(_) => false, // Always has payload
            LpMessage::SubsessionReady(_) => false, // Always has receiver_index
            LpMessage::SubsessionAbort => true,   // Empty signal
        }
    }

    pub fn len(&self) -> usize {
        match self {
            LpMessage::Busy => 0,
            LpMessage::Handshake(payload) => payload.len(),
            LpMessage::EncryptedData(payload) => payload.len(),
            LpMessage::ClientHello(payload) => payload.len(),
            LpMessage::KKTRequest(payload) => payload.len(),
            LpMessage::KKTResponse(payload) => payload.len(),
            LpMessage::ForwardPacket(payload) => payload.len(),
            LpMessage::Collision => 0,
            LpMessage::Ack => 0,
            LpMessage::SubsessionRequest => 0,
            LpMessage::SubsessionKK1(payload) => payload.len(),
            LpMessage::SubsessionKK2(payload) => payload.len(),
            LpMessage::SubsessionReady(payload) => payload.len(),
            LpMessage::SubsessionAbort => 0,
        }
    }

    pub fn typ(&self) -> MessageType {
        match self {
            LpMessage::Busy => MessageType::Busy,
            LpMessage::Handshake(_) => MessageType::Handshake,
            LpMessage::EncryptedData(_) => MessageType::EncryptedData,
            LpMessage::ClientHello(_) => MessageType::ClientHello,
            LpMessage::KKTRequest(_) => MessageType::KKTRequest,
            LpMessage::KKTResponse(_) => MessageType::KKTResponse,
            LpMessage::ForwardPacket(_) => MessageType::ForwardPacket,
            LpMessage::Collision => MessageType::Collision,
            LpMessage::Ack => MessageType::Ack,
            LpMessage::SubsessionRequest => MessageType::SubsessionRequest,
            LpMessage::SubsessionKK1(_) => MessageType::SubsessionKK1,
            LpMessage::SubsessionKK2(_) => MessageType::SubsessionKK2,
            LpMessage::SubsessionReady(_) => MessageType::SubsessionReady,
            LpMessage::SubsessionAbort => MessageType::SubsessionAbort,
        }
    }

    pub fn encode_content(&self, dst: &mut BytesMut) {
        match self {
            LpMessage::Busy => { /* No content */ }
            LpMessage::Handshake(payload) => payload.encode(dst),
            LpMessage::EncryptedData(payload) => payload.encode(dst),
            LpMessage::ClientHello(data) => data.encode(dst),
            LpMessage::KKTRequest(payload) => payload.encode(dst),
            LpMessage::KKTResponse(payload) => payload.encode(dst),
            LpMessage::ForwardPacket(data) => data.encode(dst),
            LpMessage::Collision => { /* No content */ }
            LpMessage::Ack => { /* No content */ }
            LpMessage::SubsessionRequest => { /* No content - signal only */ }
            LpMessage::SubsessionKK1(data) => data.encode(dst),
            LpMessage::SubsessionKK2(data) => data.encode(dst),
            LpMessage::SubsessionReady(data) => data.encode(dst),
            LpMessage::SubsessionAbort => { /* No content - signal only */ }
        }
    }

    /// Parse message from its type and content bytes.
    ///
    /// Used when decrypting outer-encrypted packets where the message type
    /// was encrypted along with the content.
    pub fn decode_content(content: &[u8], message_type: MessageType) -> Result<Self, LpError> {
        match message_type {
            MessageType::Busy => {
                content.ensure_empty()?;
                Ok(LpMessage::Busy)
            }
            MessageType::Handshake => Ok(LpMessage::Handshake(HandshakeData::decode(content)?)),
            MessageType::EncryptedData => Ok(LpMessage::EncryptedData(
                EncryptedDataPayload::decode(content)?,
            )),
            MessageType::ClientHello => {
                Ok(LpMessage::ClientHello(ClientHelloData::decode(content)?))
            }
            MessageType::KKTRequest => Ok(LpMessage::KKTRequest(KKTRequestData::decode(content)?)),
            MessageType::KKTResponse => {
                Ok(LpMessage::KKTResponse(KKTResponseData::decode(content)?))
            }
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
            MessageType::SubsessionRequest => {
                content.ensure_empty()?;
                Ok(LpMessage::SubsessionRequest)
            }
            MessageType::SubsessionKK1 => Ok(LpMessage::SubsessionKK1(SubsessionKK1Data::decode(
                content,
            )?)),
            MessageType::SubsessionKK2 => Ok(LpMessage::SubsessionKK2(SubsessionKK2Data::decode(
                content,
            )?)),
            MessageType::SubsessionReady => Ok(LpMessage::SubsessionReady(
                SubsessionReadyData::decode(content)?,
            )),
            MessageType::SubsessionAbort => {
                content.ensure_empty()?;
                Ok(LpMessage::SubsessionAbort)
            }
        }
    }
}

/// Helper trait for improving readability to return error if bytes content is not empty
trait EnsureEmptyContent {
    fn ensure_empty(&self) -> Result<(), LpError>;
}

impl EnsureEmptyContent for &[u8] {
    fn ensure_empty(&self) -> Result<(), LpError> {
        if !self.is_empty() {
            return Err(LpError::InvalidPayloadSize {
                expected: 0,
                actual: self.len(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::LpPacket;
    use crate::packet::{LpHeader, TRAILER_LEN};

    #[test]
    fn encoding() {
        let message = LpMessage::EncryptedData(EncryptedDataPayload(vec![11u8; 124]));

        let resp_header = LpHeader {
            protocol_version: 1,
            reserved: 0,
            receiver_idx: 0,
            counter: 0,
        };

        let packet = LpPacket {
            header: resp_header,
            message,
            trailer: [80; TRAILER_LEN],
        };

        // Just print packet for debug, will be captured in test output
        println!("{packet:?}");

        // Verify message type
        assert!(matches!(packet.message.typ(), MessageType::EncryptedData));

        // Verify correct data in message
        match &packet.message {
            LpMessage::EncryptedData(data) => {
                assert_eq!(*data, EncryptedDataPayload(vec![11u8; 124]));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_client_hello_salt_generation() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();
        let client_key = [1u8; 32];
        let client_ed25519_key = [2u8; 32];
        let hello1 =
            ClientHelloData::new_with_fresh_salt(client_key, client_ed25519_key, timestamp);
        let hello2 =
            ClientHelloData::new_with_fresh_salt(client_key, client_ed25519_key, timestamp);

        // Different salts should be generated
        assert_ne!(hello1.salt, hello2.salt);

        // But timestamps should be very close (within 1 second)
        let ts1 = hello1.extract_timestamp();
        let ts2 = hello2.extract_timestamp();
        assert!((ts1 as i64 - ts2 as i64).abs() <= 1);
    }

    #[test]
    fn test_client_hello_timestamp_extraction() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();
        let client_key = [2u8; 32];
        let client_ed25519_key = [3u8; 32];
        let hello = ClientHelloData::new_with_fresh_salt(client_key, client_ed25519_key, timestamp);

        let timestamp = hello.extract_timestamp();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Timestamp should be within 1 second of now
        assert!((timestamp as i64 - now as i64).abs() <= 1);
    }

    #[test]
    fn test_client_hello_salt_format() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();
        let client_key = [3u8; 32];
        let client_ed25519_key = [4u8; 32];
        let hello = ClientHelloData::new_with_fresh_salt(client_key, client_ed25519_key, timestamp);

        // First 8 bytes should be non-zero timestamp
        let timestamp_bytes = &hello.salt[..8];
        assert_ne!(timestamp_bytes, &[0u8; 8]);

        // Salt should be 32 bytes total
        assert_eq!(hello.salt.len(), 32);
    }
}
