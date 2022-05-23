// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::networking::error::NetworkingError;
use crate::networking::message::{Header, OffchainMessage};
use crate::networking::PROTOCOL_VERSION;
use bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

// TODO: that was fine for the purposes of DKG, we might want to increase it if it's used anywhere else
const MAX_ALLOWED_MESSAGE_LEN: u64 = 2 * 1024 * 1024;

#[derive(Debug)]
pub struct OffchainCodec;

impl<'a> Encoder<&'a OffchainMessage> for OffchainCodec {
    type Error = NetworkingError;

    fn encode(&mut self, item: &OffchainMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.encode(dst)
    }
}

impl Decoder for OffchainCodec {
    type Item = OffchainMessage;
    type Error = NetworkingError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < Header::LEN {
            // can't do much without being able to deserialize the header
            return Ok(None);
        }

        // header deserialization is simple enough to not be too cumbersome if it were to be repeated
        let header = Header::try_from_bytes(&src[..Header::LEN])?;

        if header.payload_length > MAX_ALLOWED_MESSAGE_LEN {
            return Err(NetworkingError::MessageTooLarge {
                supported: MAX_ALLOWED_MESSAGE_LEN,
                received: header.payload_length,
            });
        }

        if header.protocol_version != PROTOCOL_VERSION {
            return Err(NetworkingError::MismatchedProtocolVersion {
                expected: PROTOCOL_VERSION,
                received: header.protocol_version,
            });
        }

        if src.len() < Header::LEN + header.payload_length as usize {
            // we haven't received the entire expected message yet.
            // However, reserve enough bytes in the buffer for it
            src.reserve(Header::LEN + header.payload_length as usize - src.len());

            // We inform the Framed that we need more bytes to form the next frame.
            return Ok(None);
        }

        let payload = src[Header::LEN..Header::LEN + header.payload_length as usize].to_vec();
        src.advance(Header::LEN + header.payload_length as usize);

        Ok(Some(OffchainMessage::try_from_bytes(payload)?))
    }
}
