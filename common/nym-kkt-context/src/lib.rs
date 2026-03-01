// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use num_enum::{IntoPrimitive, TryFromPrimitive};
use nym_kkt_ciphersuite::{CIPHERSUITE_ENCODING_LEN, Ciphersuite};
use std::fmt::Display;
use thiserror::Error;

// This must be less than 4 bits
pub const KKT_VERSION: u8 = 1;
const _: () = assert!(KKT_VERSION < 1 << 4);

pub const KKT_CONTEXT_LEN: usize = 3 + CIPHERSUITE_ENCODING_LEN;

#[derive(Debug, Error)]
pub enum KKTContextEncodingError {
    #[error("KKT Message Count Limit Reached")]
    MessageCountLimitReached,

    #[error("{version} is not a valid KKT version")]
    InvalidVersion { version: u8 },

    #[error("{raw} is not a valid KKTStatus")]
    InvalidStatus { raw: u8 },

    #[error("{raw} is not a valid KKTRole")]
    InvalidRole { raw: u8 },

    #[error("{raw} is not a valid KKTMode")]
    InvalidMode { raw: u8 },

    #[error(transparent)]
    InvalidCiphersuite(#[from] nym_kkt_ciphersuite::error::KKTCiphersuiteError),
}

// bitmask used: 0b1110_0000
#[derive(Clone, Copy, PartialEq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum KKTStatus {
    Ok = 0b0000_0000,
    InvalidRequestFormat = 0b0010_0000,
    InvalidResponseFormat = 0b0100_0000,
    UnsupportedCiphersuite = 0b0110_0000,
    UnsupportedKKTVersion = 0b1000_0000,
    InvalidKey = 0b1010_0000,
    Timeout = 0b1100_0000,
    UnverifiedKEMKey = 0b1110_0000,
}

impl Display for KKTStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            KKTStatus::Ok => "Ok",
            KKTStatus::InvalidRequestFormat => "Invalid Request Format",
            KKTStatus::InvalidResponseFormat => "Invalid Response Format",
            KKTStatus::UnsupportedCiphersuite => "Unsupported Ciphersuite",
            KKTStatus::UnsupportedKKTVersion => "Unsupported KKT Version",
            KKTStatus::InvalidKey => "Invalid Key",
            KKTStatus::UnverifiedKEMKey => "Could not verify received encapsulation key",
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
}

impl KKTRole {
    pub const fn is_initiator(&self) -> bool {
        matches!(self, KKTRole::Initiator)
    }

    pub const fn is_responder(&self) -> bool {
        matches!(self, KKTRole::Responder)
    }
}

// bitmask used: 0b0001_1100
#[derive(Clone, Copy, PartialEq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum KKTMode {
    OneWay = 0b0000_0000,
    Mutual = 0b0000_0100,
}

impl KKTMode {
    pub const fn is_one_way(&self) -> bool {
        matches!(self, KKTMode::OneWay)
    }

    pub const fn is_mutual(&self) -> bool {
        matches!(self, KKTMode::Mutual)
    }
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
    pub fn new(role: KKTRole, mode: KKTMode, ciphersuite: Ciphersuite) -> Self {
        Self {
            version: KKT_VERSION,
            message_sequence: 0,
            status: KKTStatus::Ok,
            mode,
            role,
            ciphersuite,
        }
    }

    pub fn derive_responder_header(&self) -> Result<Self, KKTContextEncodingError> {
        let mut responder_header = *self;

        responder_header.increment_message_sequence_count()?;
        responder_header.role = KKTRole::Responder;

        Ok(responder_header)
    }

    pub fn increment_message_sequence_count(&mut self) -> Result<(), KKTContextEncodingError> {
        if self.message_sequence + 1 < (1 << 4) {
            self.message_sequence += 1;
            Ok(())
        } else {
            Err(KKTContextEncodingError::MessageCountLimitReached)
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
        if (self.status != KKTStatus::Ok && self.status != KKTStatus::UnverifiedKEMKey)
            ||
            // no payload
            (self.mode == KKTMode::OneWay && self.role == KKTRole::Initiator)
        {
            0
        } else {
            self.ciphersuite.kem_key_len()
        }
    }

    pub const fn header_len(&self) -> usize {
        KKT_CONTEXT_LEN
    }

    pub fn full_message_len(&self) -> usize {
        self.body_len() + self.header_len()
    }

    pub fn encode(&self) -> Result<[u8; KKT_CONTEXT_LEN], KKTContextEncodingError> {
        let mut header_bytes = [0u8; KKT_CONTEXT_LEN];
        if self.message_sequence >= 1 << 4 {
            return Err(KKTContextEncodingError::MessageCountLimitReached);
        }

        let ciphersuite_bytes = self.ciphersuite.encode();

        header_bytes[0] = (KKT_VERSION << 4) + self.message_sequence;
        header_bytes[1] = u8::from(self.status) + u8::from(self.mode) + u8::from(self.role);

        let mut i = 2;
        for b in ciphersuite_bytes.into_iter() {
            header_bytes[i] = b;
            i += 1;
        }
        header_bytes[i] = 0;
        Ok(header_bytes)
    }

    pub fn try_decode(
        header_bytes: [u8; KKT_CONTEXT_LEN],
    ) -> Result<Self, KKTContextEncodingError> {
        let kkt_version = (header_bytes[0] & 0b1111_0000) >> 4;
        let message_sequence_counter = header_bytes[0] & 0b0000_1111;

        // We only check if stuff is valid here, not necessarily if it's compatible

        if kkt_version > KKT_VERSION {
            return Err(KKTContextEncodingError::InvalidVersion {
                version: kkt_version,
            });
        }

        let raw_kkt_status = header_bytes[1] & 0b1110_0000;
        let raw_kkt_role = header_bytes[1] & 0b0000_0011;
        let raw_kkt_mode = header_bytes[1] & 0b0001_1100;

        let status = KKTStatus::try_from(raw_kkt_status).map_err(|_| {
            KKTContextEncodingError::InvalidStatus {
                raw: raw_kkt_status,
            }
        })?;
        let role = KKTRole::try_from(raw_kkt_role)
            .map_err(|_| KKTContextEncodingError::InvalidRole { raw: raw_kkt_role })?;
        let mode = KKTMode::try_from(raw_kkt_mode)
            .map_err(|_| KKTContextEncodingError::InvalidMode { raw: raw_kkt_mode })?;

        // SAFETY: we're taking exactly `CIPHERSUITE_ENCODING_LEN` bytes
        #[allow(clippy::unwrap_used)]
        let ciphersuite_bytes = header_bytes[2..2 + CIPHERSUITE_ENCODING_LEN]
            .try_into()
            .unwrap();

        Ok(KKTContext {
            version: kkt_version,
            status,
            mode,
            role,
            ciphersuite: Ciphersuite::decode(ciphersuite_bytes)?,
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
            Ciphersuite::decode([1, 1, 0, 0]).unwrap(),
        );
        let encoded = valid_context.encode().unwrap();
        let decoded = KKTContext::try_decode(encoded).unwrap();

        assert_eq!(decoded, valid_context);
    }
}
