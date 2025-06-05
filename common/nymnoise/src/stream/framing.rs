// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::NoisePattern;
use crate::error::NoiseError;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use nym_noise_keys::NoiseVersion;
use strum::FromRepr;

#[derive(Debug)]
pub struct NymNoiseFrame {
    pub header: NymNoiseHeader,
    pub data: Bytes,
}

impl NymNoiseFrame {
    pub fn new_handshake_frame(
        data: Bytes,
        version: NoiseVersion,
        pattern: NoisePattern,
    ) -> Result<Self, NoiseError> {
        if data.len() > u16::MAX as usize {
            return Err(NoiseError::HandshakeTooBig { size: data.len() });
        }

        Ok(NymNoiseFrame {
            header: NymNoiseHeader {
                version,
                noise_pattern: pattern,
                message_type: NymNoiseMessageType::Handshake,
                data_len: data.len() as u16,
            },
            data,
        })
    }

    pub fn new_data_frame(
        data: Bytes,
        version: NoiseVersion,
        pattern: NoisePattern,
    ) -> Result<Self, NoiseError> {
        if data.len() > u16::MAX as usize {
            return Err(NoiseError::HandshakeTooBig { size: data.len() });
        }

        Ok(NymNoiseFrame {
            header: NymNoiseHeader {
                version,
                noise_pattern: pattern,
                message_type: NymNoiseMessageType::Data,
                data_len: data.len() as u16,
            },
            data,
        })
    }

    pub fn version(&self) -> NoiseVersion {
        self.header.version
    }

    pub fn is_handshake_message(&self) -> bool {
        self.header.is_handshake_message()
    }

    pub fn is_data_message(&self) -> bool {
        self.header.is_data_message()
    }

    pub fn noise_pattern(&self) -> NoisePattern {
        self.header.noise_pattern
    }
}

#[derive(Debug, Copy, Clone, FromRepr)]
#[repr(u8)]
#[non_exhaustive]
pub enum NymNoiseMessageType {
    Handshake = 0,
    Data = 1,
}

#[derive(Debug, Clone, Copy)]
pub struct NymNoiseHeader {
    pub version: NoiseVersion,
    pub noise_pattern: NoisePattern,
    pub message_type: NymNoiseMessageType,
    pub data_len: u16,
}

impl NymNoiseHeader {
    pub(crate) const SIZE: usize = 8;

    pub fn is_handshake_message(&self) -> bool {
        matches!(self.message_type, NymNoiseMessageType::Handshake)
    }

    pub fn is_data_message(&self) -> bool {
        matches!(self.message_type, NymNoiseMessageType::Data)
    }

    // 0 1 2 3 4 5 6 7 8
    // +-+-+-+-+-+-+-+-+
    // |V|P|T|Len| Res.|
    // +-+-+-+-+-+-+-+-+
    pub(crate) fn encode(&self, dst: &mut BytesMut) {
        dst.reserve(Self::SIZE);

        // byte 0
        dst.put_u8(self.version.into());

        // byte 1
        dst.put_u8(self.noise_pattern as u8);

        // byte 2
        dst.put_u8(self.message_type as u8);

        // byte 3-4
        dst.put_u16(self.data_len);

        // byte 5-7 (RESERVED):
        dst.extend_from_slice(&[0u8; 3])
    }

    pub(crate) fn decode(src: &mut BytesMut) -> Result<Option<Self>, NoiseError> {
        if src.len() < Self::SIZE {
            // can't do anything if we don't have enough bytes - but reserve enough for the next call
            src.reserve(Self::SIZE);
            return Ok(None);
        }

        let version = src.get_u8();
        let pattern = src.get_u8();
        let message_type = src.get_u8();
        let data_len = src.get_u16();

        // reserved
        src.advance(3);

        let version = NoiseVersion::from(version);

        // here, based on versions, we could do vary the further parsing
        // match version {
        //     NoiseVersion::V1 => {}
        //     NoiseVersion::Unknown(_) => {}
        // }

        let noise_pattern = NoisePattern::from_repr(pattern)
            .ok_or(NoiseError::UnknownPattern { encoded: pattern })?;
        let message_type =
            NymNoiseMessageType::from_repr(message_type).ok_or(NoiseError::UnknownMessageType {
                encoded: message_type,
            })?;

        Ok(Some(NymNoiseHeader {
            version,
            noise_pattern,
            message_type,
            data_len,
        }))
    }
}
