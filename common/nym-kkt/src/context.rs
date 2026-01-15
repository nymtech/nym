// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{KKT_VERSION, ciphersuite::Ciphersuite, error::KKTError, frame::KKT_SESSION_ID_LEN};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::fmt::Display;

pub const KKT_CONTEXT_LEN: usize = 7;

// bitmask used: 0b1110_0000
#[derive(Clone, Copy, PartialEq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum KKTStatus {
    Ok = 0b0000_0000,
    InvalidRequestFormat = 0b0010_0000,
    InvalidResponseFormat = 0b0100_0000,
    InvalidSignature = 0b0110_0000,
    UnsupportedCiphersuite = 0b1000_0000,
    UnsupportedKKTVersion = 0b1010_0000,
    InvalidKey = 0b1100_0000,
    Timeout = 0b1110_0000,
}

impl Display for KKTStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            KKTStatus::Ok => "Ok",
            KKTStatus::InvalidRequestFormat => "Invalid Request Format",
            KKTStatus::InvalidResponseFormat => "Invalid Response Format",
            KKTStatus::InvalidSignature => "Invalid Signature",
            KKTStatus::UnsupportedCiphersuite => "Unsupported Ciphersuite",
            KKTStatus::UnsupportedKKTVersion => "Unsupported KKT Version",
            KKTStatus::InvalidKey => "Invalid Key",
            KKTStatus::Timeout => "Timeout",
        })
    }
}

// bitmask used: 0b0000_0011
#[derive(Clone, Copy, PartialEq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum KKTRole {
    Initiator = 0b0000_0000,
    Responder = 0b0000_0001,
    AnonymousInitiator = 0b0000_0010,
}

// bitmask used: 0b0001_1100
#[derive(Clone, Copy, PartialEq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum KKTMode {
    OneWay = 0b0000_0000,
    Mutual = 0b0000_0100,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct KKTContext {
    version: u8,
    message_sequence: u8,
    status: KKTStatus,
    mode: KKTMode,
    role: KKTRole,
    ciphersuite: Ciphersuite,
}
impl KKTContext {
    pub fn new(role: KKTRole, mode: KKTMode, ciphersuite: Ciphersuite) -> Result<Self, KKTError> {
        if role == KKTRole::AnonymousInitiator && mode != KKTMode::OneWay {
            return Err(KKTError::IncompatibilityError {
                info: "Anonymous Initiator can only use OneWay mode",
            });
        }
        Ok(Self {
            version: KKT_VERSION,
            message_sequence: 0,
            status: KKTStatus::Ok,
            mode,
            role,
            ciphersuite,
        })
    }

    pub fn derive_responder_header(&self) -> Result<Self, KKTError> {
        let mut responder_header = *self;

        responder_header.increment_message_sequence_count()?;
        responder_header.role = KKTRole::Responder;

        Ok(responder_header)
    }

    pub fn increment_message_sequence_count(&mut self) -> Result<(), KKTError> {
        if self.message_sequence + 1 < (1 << 4) {
            self.message_sequence += 1;
            Ok(())
        } else {
            Err(KKTError::MessageCountLimitReached)
        }
    }

    pub fn update_status(&mut self, status: KKTStatus) {
        self.status = status;
    }
    pub fn version(&self) -> u8 {
        self.version
    }
    pub fn status(&self) -> KKTStatus {
        self.status
    }
    pub fn ciphersuite(&self) -> Ciphersuite {
        self.ciphersuite
    }
    pub fn role(&self) -> KKTRole {
        self.role
    }
    pub fn mode(&self) -> KKTMode {
        self.mode
    }

    pub fn body_len(&self) -> usize {
        if self.status != KKTStatus::Ok
            || (self.mode == KKTMode::OneWay
                && (self.role == KKTRole::Initiator || self.role == KKTRole::AnonymousInitiator))
        {
            0
        } else {
            self.ciphersuite.kem_key_len()
        }
    }

    pub fn signature_len(&self) -> usize {
        match self.role {
            KKTRole::Initiator | KKTRole::Responder => self.ciphersuite.signature_len(),
            KKTRole::AnonymousInitiator => 0,
        }
    }

    pub const fn header_len(&self) -> usize {
        KKT_CONTEXT_LEN
    }

    pub const fn session_id_len(&self) -> usize {
        // match self.role {
        //     KKTRole::Initiator | KKTRole::Responder => SESSION_ID_LENGTH,
        // It doesn't make sense to send a session_id if we send messages in the clear
        //     KKTRole::AnonymousInitiator => 0,
        // }
        KKT_SESSION_ID_LEN
    }

    pub fn full_message_len(&self) -> usize {
        self.body_len() + self.signature_len() + self.header_len() + self.session_id_len()
    }

    pub fn encode(&self) -> Result<Vec<u8>, KKTError> {
        let mut header_bytes: Vec<u8> = Vec::with_capacity(KKT_CONTEXT_LEN);
        if self.message_sequence >= 1 << 4 {
            return Err(KKTError::MessageCountLimitReached);
        }

        header_bytes.push((KKT_VERSION << 4) + self.message_sequence);
        header_bytes.push(u8::from(self.status) + u8::from(self.mode) + u8::from(self.role));

        header_bytes.extend_from_slice(&self.ciphersuite.encode());
        header_bytes.push(0);
        Ok(header_bytes)
    }

    pub fn try_decode(header_bytes: &[u8]) -> Result<Self, KKTError> {
        if header_bytes.len() != KKT_CONTEXT_LEN {
            return Err(KKTError::FrameDecodingError {
                info: format!(
                    "Header - Invalid Header Length: actual: {} != expected: {}",
                    header_bytes.len(),
                    KKT_CONTEXT_LEN
                ),
            });
        }

        let kkt_version = (header_bytes[0] & 0b1111_0000) >> 4;
        let message_sequence_counter = header_bytes[0] & 0b0000_1111;

        // We only check if stuff is valid here, not necessarily if it's compatible

        if kkt_version > KKT_VERSION {
            return Err(KKTError::FrameDecodingError {
                info: format!("Header - Invalid KKT Version: {kkt_version}"),
            });
        }

        let raw_kkt_status = header_bytes[1] & 0b1110_0000;
        let raw_kkt_role = header_bytes[1] & 0b0000_0011;
        let raw_kkt_mode = header_bytes[1] & 0b0001_1100;

        let status =
            KKTStatus::try_from(raw_kkt_status).map_err(|_| KKTError::FrameDecodingError {
                info: format!("Header - Invalid KKT Status: {raw_kkt_status}"),
            })?;
        let role = KKTRole::try_from(raw_kkt_role).map_err(|_| KKTError::FrameDecodingError {
            info: format!("Header - Invalid KKT Role: {raw_kkt_role}"),
        })?;
        let mode = KKTMode::try_from(raw_kkt_mode).map_err(|_| KKTError::FrameDecodingError {
            info: format!("Header - Invalid KKT Mode: {raw_kkt_mode}"),
        })?;

        Ok(KKTContext {
            version: kkt_version,
            status,
            mode,
            role,
            ciphersuite: Ciphersuite::decode(&header_bytes[2..6])?,
            message_sequence: message_sequence_counter,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kkt_context_encoding() {
        let valid_context = KKTContext::new(
            KKTRole::Initiator,
            KKTMode::Mutual,
            Ciphersuite::decode(&[255, 1, 0, 0]).unwrap(),
        )
        .unwrap();
        let encoded = valid_context.encode().unwrap();
        let decoded = KKTContext::try_decode(&encoded).unwrap();

        assert_eq!(decoded, valid_context);
    }
}
