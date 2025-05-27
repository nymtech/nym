// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::{Bytes, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

pub struct NymNoiseCodec {
    // right now do the naive thing of reusing the existing codec
    inner: LengthDelimitedCodec,
}

impl NymNoiseCodec {
    pub fn new() -> Self {
        NymNoiseCodec {
            inner: LengthDelimitedCodec::builder()
                .length_field_type::<u16>()
                .new_codec(),
        }
    }
}

impl Decoder for NymNoiseCodec {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode(src)
    }
}

impl Encoder<Bytes> for NymNoiseCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.inner.encode(item, dst)
    }
}
