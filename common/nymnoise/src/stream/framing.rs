// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::NoisePattern;
use crate::error::NoiseError;
use bytes::{BufMut, Bytes, BytesMut};
use strum::FromRepr;

pub struct NymNoiseFramedItem {
    pub header: NymNoiseHeader,
    pub data: Bytes,
}

pub const CURRENT_NYM_NOISE_VERSION: u8 = 1;

#[derive(Debug, Copy, Clone, FromRepr)]
#[repr(u8)]
#[non_exhaustive]
pub enum NymNoiseVersion {
    Initial = 1,
}

pub struct NymNoiseHeader {
    pub version: NymNoiseVersion,
    pub noise_pattern: NoisePattern,

    // message type?
    pub data_len: u16,
}

impl NymNoiseHeader {
    const SIZE: usize = 8;

    pub(crate) fn encode(&self, dst: &mut BytesMut) {
        dst.reserve(Self::SIZE);

        // byte 0
        dst.put_u8(self.version as u8);

        // byte 1
        dst.put_u8(self.noise_pattern as u8);

        // byte 2-3
        dst.put_u16(self.data_len);

        // byte 4-7 (RESERVED):
        dst.extend_from_slice(&[0u8; 4])
    }

    pub(crate) fn decode(src: &mut BytesMut) -> Result<Option<Self>, NoiseError> {
        if src.len() < Self::SIZE {
            // can't do anything if we don't have enough bytes - but reserve enough for the next call
            src.reserve(Self::SIZE);
            return Ok(None);
        }

        todo!()
    }
}
