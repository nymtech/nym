// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NoiseError;
use crate::stream::framing::{NymNoiseFrame, NymNoiseHeader};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug, Clone, Copy)]
enum DecodeState {
    Header,
    Payload(NymNoiseHeader),
}

pub struct NymNoiseCodec {
    state: DecodeState,
}

impl NymNoiseCodec {
    pub fn new() -> Self {
        NymNoiseCodec {
            state: DecodeState::Header,
        }
    }

    fn decode_header(&self, src: &mut BytesMut) -> Result<Option<NymNoiseHeader>, NoiseError> {
        if src.len() < NymNoiseHeader::SIZE {
            // Not enough data
            return Ok(None);
        }

        // note: successful call to 'decode' advances the buffer by NymNoiseHeader::SIZE
        let Some(header) = NymNoiseHeader::decode(src)? else {
            return Ok(None);
        };

        Ok(Some(header))
    }

    fn decode_data(&self, n: usize, src: &mut BytesMut) -> Option<BytesMut> {
        // At this point, the buffer has already had the required capacity
        // reserved. All there is to do is read.
        if src.len() < n {
            return None;
        }

        Some(src.split_to(n))
    }
}

impl Decoder for NymNoiseCodec {
    type Item = NymNoiseFrame;
    type Error = NoiseError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let header = match self.state {
            DecodeState::Header => match self.decode_header(src)? {
                None => return Ok(None),
                Some(header) => {
                    self.state = DecodeState::Payload(header);
                    header
                }
            },
            DecodeState::Payload(header) => header,
        };

        let Some(data) = self.decode_data(header.data_len as usize, src) else {
            return Ok(None);
        };

        // Update the decode state
        self.state = DecodeState::Header;

        // make sure the buffer has enough space to read the next header
        src.reserve(NymNoiseHeader::SIZE);

        Ok(Some(NymNoiseFrame {
            header,
            data: data.freeze(),
        }))
    }
}

impl Encoder<NymNoiseFrame> for NymNoiseCodec {
    type Error = NoiseError;

    fn encode(&mut self, frame: NymNoiseFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        frame.header.encode(dst);
        dst.put_slice(frame.data.as_ref());
        Ok(())
    }
}
