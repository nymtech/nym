// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;

use crate::{ciphersuite::Ciphersuite, error::KKTError, frame::KKT_SESSION_ID_LEN, KKT_VERSION};

pub const KKT_CONTEXT_LEN: usize = 7;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KKTStatus {
    Ok,
    InvalidRequestFormat,
    InvalidResponseFormat,
    InvalidSignature,
    UnsupportedCiphersuite,
    UnsupportedKKTVersion,
    InvalidKey,
    Timeout,
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
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KKTRole {
    Initiator,
    AnonymousInitiator,
    Responder,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KKTMode {
    OneWay,
    Mutual,
}

#[derive(Copy, Clone, Debug)]
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

    pub fn header_len(&self) -> usize {
        KKT_CONTEXT_LEN
    }

    pub fn session_id_len(&self) -> usize {
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

        header_bytes.push(
            match self.status {
                KKTStatus::Ok => 0,
                KKTStatus::InvalidRequestFormat => 0b001_000_00,
                KKTStatus::InvalidResponseFormat => 0b010_000_00,
                KKTStatus::InvalidSignature => 0b011_000_00,
                KKTStatus::UnsupportedCiphersuite => 0b100_000_00,
                KKTStatus::UnsupportedKKTVersion => 0b101_000_00,
                KKTStatus::InvalidKey => 0b110_000_00,
                KKTStatus::Timeout => 0b111_000_00,
            } + match self.mode {
                KKTMode::OneWay => 0,
                KKTMode::Mutual => 0b000_001_00,
            } + match self.role {
                KKTRole::Initiator => 0,
                KKTRole::Responder => 1,
                KKTRole::AnonymousInitiator => 2,
            },
        );

        header_bytes.extend_from_slice(&self.ciphersuite.encode());
        header_bytes.push(0);
        Ok(header_bytes)
    }

    pub fn try_decode(header_bytes: &[u8]) -> Result<Self, KKTError> {
        if header_bytes.len() == KKT_CONTEXT_LEN {
            let kkt_version = header_bytes[0] & 0b1111_0000;

            let message_sequence_counter = header_bytes[0] & 0b0000_1111;

            // We only check if stuff is valid here, not necessarily if it's compatible

            if (kkt_version >> 4) > KKT_VERSION {
                return Err(KKTError::FrameDecodingError {
                    info: format!("Header - Invalid KKT Version: {}", kkt_version >> 4),
                });
            }

            let status = match header_bytes[1] & 0b111_000_00 {
                0 => KKTStatus::Ok,
                0b001_000_00 => KKTStatus::InvalidRequestFormat,
                0b010_000_00 => KKTStatus::InvalidResponseFormat,
                0b011_000_00 => KKTStatus::InvalidSignature,
                0b100_000_00 => KKTStatus::UnsupportedCiphersuite,
                0b101_000_00 => KKTStatus::UnsupportedKKTVersion,
                0b110_000_00 => KKTStatus::InvalidKey,
                0b111_000_00 => KKTStatus::Timeout,
                _ => {
                    return Err(KKTError::FrameDecodingError {
                        info: format!(
                            "Header - Invalid KKT Status: {}",
                            header_bytes[1] & 0b111_000_00
                        ),
                    })
                }
            };

            let role = match header_bytes[1] & 0b000_000_11 {
                0 => KKTRole::Initiator,
                1 => KKTRole::Responder,
                2 => KKTRole::AnonymousInitiator,
                _ => {
                    return Err(KKTError::FrameDecodingError {
                        info: format!(
                            "Header - Invalid KKT Role: {}",
                            header_bytes[1] & 0b000_000_11
                        ),
                    })
                }
            };

            let mode = match (header_bytes[1] & 0b000_111_00) >> 2 {
                0 => KKTMode::OneWay,
                1 => KKTMode::Mutual,
                _ => {
                    return Err(KKTError::FrameDecodingError {
                        info: format!(
                            "Header - Invalid KKT Mode: {}",
                            (header_bytes[1] & 0b000_111_00) >> 2
                        ),
                    })
                }
            };

            Ok(KKTContext {
                version: kkt_version,
                status,
                mode,
                role,
                ciphersuite: Ciphersuite::decode(&header_bytes[2..6])?,
                message_sequence: message_sequence_counter,
            })
        } else {
            Err(KKTError::FrameDecodingError {
                info: format!(
                    "Header - Invalid Header Length: actual: {} != expected: {}",
                    header_bytes.len(),
                    KKT_CONTEXT_LEN
                ),
            })
        }
    }
}
