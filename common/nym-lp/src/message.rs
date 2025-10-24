use std::fmt::{self, Display};

// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};

/// Data structure for the ClientHello message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHelloData {
    /// Client's LP x25519 public key (32 bytes)
    pub client_lp_public_key: [u8; 32],
    /// Protocol version for future compatibility
    pub protocol_version: u8,
    /// Salt for PSK derivation (32 bytes: 8-byte timestamp + 24-byte nonce)
    pub salt: [u8; 32],
}

impl ClientHelloData {
    /// Generates a new ClientHelloData with fresh salt.
    ///
    /// Salt format: 8 bytes timestamp (u64 LE) + 24 bytes random nonce
    ///
    /// # Arguments
    /// * `client_lp_public_key` - Client's x25519 public key
    /// * `protocol_version` - Protocol version number
    pub fn new_with_fresh_salt(client_lp_public_key: [u8; 32], protocol_version: u8) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Generate salt: timestamp + nonce
        let mut salt = [0u8; 32];

        // First 8 bytes: current timestamp as u64 little-endian
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();
        salt[..8].copy_from_slice(&timestamp.to_le_bytes());

        // Last 24 bytes: random nonce
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut salt[8..]);

        Self {
            client_lp_public_key,
            protocol_version,
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
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageType {
    Busy = 0x0000,
    Handshake = 0x0001,
    EncryptedData = 0x0002,
    ClientHello = 0x0003,
}

impl MessageType {
    pub(crate) fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0000 => Some(MessageType::Busy),
            0x0001 => Some(MessageType::Handshake),
            0x0002 => Some(MessageType::EncryptedData),
            0x0003 => Some(MessageType::ClientHello),
            _ => None,
        }
    }

    pub fn to_u16(&self) -> u16 {
        match self {
            MessageType::Busy => 0x0000,
            MessageType::Handshake => 0x0001,
            MessageType::EncryptedData => 0x0002,
            MessageType::ClientHello => 0x0003,
        }
    }
}

#[derive(Debug, Clone)]
pub enum LpMessage {
    Busy,
    Handshake(Vec<u8>),
    EncryptedData(Vec<u8>),
    ClientHello(ClientHelloData),
}

impl Display for LpMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LpMessage::Busy => write!(f, "Busy"),
            LpMessage::Handshake(_) => write!(f, "Handshake"),
            LpMessage::EncryptedData(_) => write!(f, "EncryptedData"),
            LpMessage::ClientHello(_) => write!(f, "ClientHello"),
        }
    }
}

impl LpMessage {
    pub fn payload(&self) -> &[u8] {
        match self {
            LpMessage::Busy => &[],
            LpMessage::Handshake(payload) => payload,
            LpMessage::EncryptedData(payload) => payload,
            LpMessage::ClientHello(_) => &[], // Structured data, serialized in encode_content
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            LpMessage::Busy => true,
            LpMessage::Handshake(payload) => payload.is_empty(),
            LpMessage::EncryptedData(payload) => payload.is_empty(),
            LpMessage::ClientHello(_) => false, // Always has data
        }
    }

    pub fn len(&self) -> usize {
        match self {
            LpMessage::Busy => 0,
            LpMessage::Handshake(payload) => payload.len(),
            LpMessage::EncryptedData(payload) => payload.len(),
            LpMessage::ClientHello(_) => 65, // 32 bytes key + 1 byte version + 32 bytes salt
        }
    }

    pub fn typ(&self) -> MessageType {
        match self {
            LpMessage::Busy => MessageType::Busy,
            LpMessage::Handshake(_) => MessageType::Handshake,
            LpMessage::EncryptedData(_) => MessageType::EncryptedData,
            LpMessage::ClientHello(_) => MessageType::ClientHello,
        }
    }

    pub fn encode_content(&self, dst: &mut BytesMut) {
        match self {
            LpMessage::Busy => { /* No content */ }
            LpMessage::Handshake(payload) => {
                dst.put_slice(payload);
            }
            LpMessage::EncryptedData(payload) => {
                dst.put_slice(payload);
            }
            LpMessage::ClientHello(data) => {
                // Serialize ClientHelloData using bincode
                let serialized =
                    bincode::serialize(data).expect("Failed to serialize ClientHelloData");
                dst.put_slice(&serialized);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{LpHeader, TRAILER_LEN};
    use crate::LpPacket;

    #[test]
    fn encoding() {
        let message = LpMessage::EncryptedData(vec![11u8; 124]);

        let resp_header = LpHeader {
            protocol_version: 1,
            session_id: 0,
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
                assert_eq!(*data, vec![11u8; 124]);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_client_hello_salt_generation() {
        let client_key = [1u8; 32];
        let hello1 = ClientHelloData::new_with_fresh_salt(client_key, 1);
        let hello2 = ClientHelloData::new_with_fresh_salt(client_key, 1);

        // Different salts should be generated
        assert_ne!(hello1.salt, hello2.salt);

        // But timestamps should be very close (within 1 second)
        let ts1 = hello1.extract_timestamp();
        let ts2 = hello2.extract_timestamp();
        assert!((ts1 as i64 - ts2 as i64).abs() <= 1);
    }

    #[test]
    fn test_client_hello_timestamp_extraction() {
        let client_key = [2u8; 32];
        let hello = ClientHelloData::new_with_fresh_salt(client_key, 1);

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
        let client_key = [3u8; 32];
        let hello = ClientHelloData::new_with_fresh_salt(client_key, 1);

        // First 8 bytes should be non-zero timestamp
        let timestamp_bytes = &hello.salt[..8];
        assert_ne!(timestamp_bytes, &[0u8; 8]);

        // Salt should be 32 bytes total
        assert_eq!(hello.salt.len(), 32);
    }
}
