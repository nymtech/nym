use std::io;

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use super::packet::KcpPacket;

/// Our codec for encoding/decoding KCP packets
#[derive(Debug, Default)]
pub struct KcpCodec;

impl Decoder for KcpCodec {
    type Item = KcpPacket;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // We simply delegate to `KcpPacket::decode`
        KcpPacket::decode(src).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl Encoder<KcpPacket> for KcpCodec {
    type Error = io::Error;

    fn encode(&mut self, item: KcpPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // We just call `item.encode` to append the bytes
        item.encode(dst);
        Ok(())
    }
}
