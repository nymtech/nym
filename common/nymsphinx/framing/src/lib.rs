// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use nymsphinx_chunking::packet_sizes::{InvalidPacketSize, PacketSize};
use bytes::{Buf, BufMut, BytesMut};
use nymsphinx_types::SphinxPacket;
use std::convert::TryFrom;
use std::io;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug)]
pub enum SphinxCodecError {
    InvalidPacketSize,
    MalformedSphinxPacket,
    IoError(io::Error),
}

impl From<io::Error> for SphinxCodecError {
    fn from(err: io::Error) -> Self {
        SphinxCodecError::IoError(err)
    }
}

impl Into<io::Error> for SphinxCodecError {
    fn into(self) -> io::Error {
        match self {
            SphinxCodecError::InvalidPacketSize => {
                io::Error::new(io::ErrorKind::InvalidInput, "invalid packet size")
            }
            SphinxCodecError::MalformedSphinxPacket => {
                io::Error::new(io::ErrorKind::InvalidData, "malformed packet")
            }
            SphinxCodecError::IoError(err) => err,
        }
    }
}

impl From<InvalidPacketSize> for SphinxCodecError {
    fn from(_: InvalidPacketSize) -> Self {
        SphinxCodecError::InvalidPacketSize
    }
}

// The SphinxCodec is an extremely simple one, u8 representing one of valid packet
// lengths followed by the actual framed packet
pub struct SphinxCodec;

impl Encoder<SphinxPacket> for SphinxCodec {
    type Error = SphinxCodecError;

    fn encode(&mut self, item: SphinxPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let packet_bytes = item.to_bytes();
        let packet_length = packet_bytes.len();
        let packet_size = PacketSize::get_type(packet_length)?;
        dst.reserve(1 + packet_size.size());
        dst.put_u8(packet_size as u8);
        dst.put(packet_bytes.as_ref());
        Ok(())
    }
}

impl Decoder for SphinxCodec {
    type Item = SphinxPacket;
    type Error = SphinxCodecError;

    //https://docs.rs/tokio-util/0.3.1/tokio_util/codec/trait.Decoder.html
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            // can't do anything if we have no bytes
            return Ok(None);
        }
        // we at least have a single byte in the buffer, so we can read the expected
        // length of the sphinx packet
        let packet_len_flag = src[0];
        let packet_len = PacketSize::try_from(packet_len_flag)?;

        let frame_len = packet_len.size() + 1; // one is due to the flag taking the space
        if src.len() < frame_len {
            // we don't have enough bytes to read the entire frame
            src.reserve(frame_len);
            return Ok(None);
        }
        // we advance the buffer beyond the flag
        src.advance(1);
        let sphinx_packet_bytes = src.split_to(packet_len.size());
        let sphinx_packet = match SphinxPacket::from_bytes(&sphinx_packet_bytes) {
            Ok(sphinx_packet) => sphinx_packet,
            // here it could be debatable whether stream is corrupt or not,
            // but let's go with the safer approach and assume it is.
            Err(_) => return Err(SphinxCodecError::MalformedSphinxPacket),
        };

        // As per docs:
        // Before returning from the function, implementations should ensure that the buffer
        // has appropriate capacity in anticipation of future calls to decode.
        // Failing to do so leads to inefficiency.

        // if we have at least one more byte available, we can reserve enough bytes for
        // the entire next frame
        if !src.is_empty() {
            let next_packet_len = match PacketSize::try_from(src[0]) {
                Ok(next_packet_len) => next_packet_len,
                // the next frame will be malformed but let's leave handling the error to the next
                // call to 'decode', as presumably, the current sphinx packet is still valid
                Err(_) => return Ok(Some(sphinx_packet)),
            };
            let next_frame_len = next_packet_len.size() + 1;
            src.reserve(next_frame_len);
        }

        Ok(Some(sphinx_packet))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    #[allow(dead_code)]
    fn consume(
        codec: &mut SphinxCodec,
        bytes: &mut BytesMut,
    ) -> Vec<Result<Option<SphinxPacket>, SphinxCodecError>> {
        let mut result = Vec::new();
        loop {
            match codec.decode(bytes) {
                Ok(None) => {
                    break;
                }
                output => result.push(output),
            }
        }
        return result;
    }
}
