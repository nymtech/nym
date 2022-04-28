// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::dkg::networking::message::{Header, OffchainDkgMessage};
use crate::dkg::networking::PROTOCOL_VERSION;
use bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

const MAX_ALLOWED_MESSAGE_LEN: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub struct DkgCodec;

impl Encoder<OffchainDkgMessage> for DkgCodec {
    type Error = DkgError;

    fn encode(&mut self, item: OffchainDkgMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.encode(dst);
        Ok(())
    }
}

impl Decoder for DkgCodec {
    type Item = OffchainDkgMessage;
    type Error = DkgError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < Header::LEN {
            // can't do much without being able to deserialize the header
            return Ok(None);
        }

        // header deserialization is simple enough to not be too cumbersome if it were to be repeated
        let header = Header::try_from_bytes(&src[..Header::LEN])?;

        if header.payload_length as usize > MAX_ALLOWED_MESSAGE_LEN {
            todo!("return an error here")
        }

        if header.protocol_version != PROTOCOL_VERSION {
            todo!("return another error here")
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

        Ok(Some(OffchainDkgMessage::try_from_bytes(
            payload,
            header.message_type,
        )?))
    }
}
