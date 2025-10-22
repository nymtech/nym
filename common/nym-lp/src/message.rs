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
            LpMessage::ClientHello(_) => 33, // 32 bytes key + 1 byte version
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
                let serialized = bincode::serialize(data)
                    .expect("Failed to serialize ClientHelloData");
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
}
