use std::time::Duration;

use bytes::{Buf, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] std::io::Error),
}

pub const BUFFER_TIMEOUT: Duration = Duration::from_millis(20);

// TODO: increase this to make max out effective sphinx payload size. Sphinx packets also carry the
// MixAck so that's why we can't just use 2kb.
pub const MAX_PACKET_SIZE: usize = 1500;

// Each IP packet is prefixed by a 2 byte length prefix
const LENGTH_PREFIX_SIZE: usize = 2;

// Tokio codec for bundling multiple IP packets into one buffer that is at most 1500 bytes long.
// These packets are separated by a 2 byte length prefix. We need a timer so that we don't wait too
// long for the buffer to fill up, since this kills latency.
pub struct MultiIpPacketCodec {
    buffer: BytesMut,
}

impl MultiIpPacketCodec {
    pub fn new() -> Self {
        MultiIpPacketCodec {
            buffer: BytesMut::new(),
        }
    }

    pub fn bundle_one_packet(packet: Bytes) -> Bytes {
        let mut bundled_packets = BytesMut::new();
        bundled_packets.extend_from_slice(&(packet.len() as u16).to_be_bytes());
        bundled_packets.extend_from_slice(&packet);
        bundled_packets.freeze()
    }
}

impl Default for MultiIpPacketCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// The packet that we encode and decode with the MultiIpPacketCodec into bundled multi-ip packets.
/// The data here is the actual IP packet that we want to send, not the bundled packets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IprPacket {
    Data(Bytes),
    Flush,
}

impl IprPacket {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            IprPacket::Data(bytes) => bytes.as_ref(),
            IprPacket::Flush => &[],
        }
    }

    pub fn into_bytes(self) -> Bytes {
        match self {
            IprPacket::Data(bytes) => bytes,
            IprPacket::Flush => Bytes::new(),
        }
    }
}

impl From<Bytes> for IprPacket {
    fn from(bytes: Bytes) -> Self {
        IprPacket::Data(bytes)
    }
}

impl From<Vec<u8>> for IprPacket {
    fn from(bytes: Vec<u8>) -> Self {
        IprPacket::Data(Bytes::from(bytes))
    }
}

impl Encoder<IprPacket> for MultiIpPacketCodec {
    type Error = Error;

    fn encode(&mut self, packet: IprPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let packet = match packet {
            IprPacket::Flush => {
                dst.extend_from_slice(&self.buffer);
                self.buffer = BytesMut::new();
                return Ok(());
            }
            IprPacket::Data(packet) => packet,
        };

        let packet_size = packet.len();

        // If the existing buffer is empty, and the packet is too large, send it directly
        if self.buffer.is_empty() && packet_size + LENGTH_PREFIX_SIZE > MAX_PACKET_SIZE {
            // Add the packet size
            dst.extend_from_slice(&(packet_size as u16).to_be_bytes());
            // Add the packet to the buffer
            dst.extend_from_slice(&packet);
            return Ok(());
        }

        // If the packet doesn't fit in the existing buffer, send what we have now in the buffer
        // and then add it to the next buffer
        if self.buffer.len() + packet_size + LENGTH_PREFIX_SIZE > MAX_PACKET_SIZE {
            // Send the existing buffer
            dst.extend_from_slice(&self.buffer);
            // Start a new buffer
            self.buffer = BytesMut::new();
        }

        // Add the packet size
        self.buffer
            .extend_from_slice(&(packet_size as u16).to_be_bytes());
        // Add the packet to the buffer
        self.buffer.extend_from_slice(&packet);

        Ok(())
    }
}

impl Decoder for MultiIpPacketCodec {
    type Item = IprPacket;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < LENGTH_PREFIX_SIZE {
            // Not enough bytes to read the length prefix
            return Ok(None);
        }

        let packet_size = u16::from_be_bytes([src[0], src[1]]) as usize;

        if src.len() < packet_size + LENGTH_PREFIX_SIZE {
            // Not enough bytes to read the packet
            return Ok(None);
        }

        // Remove the length prefix
        src.advance(LENGTH_PREFIX_SIZE);

        // Read the packet
        let packet = src.split_to(packet_size);

        Ok(Some(IprPacket::Data(packet.freeze())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_ip_packet_codec_max_packet_size() {
        let mut codec = MultiIpPacketCodec::new();
        let mut buffer = BytesMut::new();

        // A packet size that is large enough that two packets won't fit in the buffer
        const PACKET_SIZE: usize = MAX_PACKET_SIZE - 100;

        let packet1 = IprPacket::from(Bytes::from_static(&[0u8; PACKET_SIZE]));
        let packet2 = IprPacket::from(Bytes::from_static(&[0u8; PACKET_SIZE]));

        codec.encode(packet1.clone(), &mut buffer).unwrap();
        assert_eq!(buffer.len(), 0);

        codec.encode(packet2.clone(), &mut buffer).unwrap();
        assert_eq!(buffer.len(), LENGTH_PREFIX_SIZE + PACKET_SIZE);

        // First is the length prefix
        assert_eq!(buffer[..2], (PACKET_SIZE as u16).to_be_bytes());
        // Next is the packet
        assert_eq!(&buffer[2..], packet1.as_bytes());
    }

    #[test]
    fn encode_and_then_decode() {
        let mut codec = MultiIpPacketCodec::new();
        let mut buffer = BytesMut::new();

        let packet = IprPacket::from(Bytes::from_static(&[0u8; 1000]));
        codec.encode(packet.clone(), &mut buffer).unwrap();
        codec.encode(packet.clone(), &mut buffer).unwrap();

        let mut decoded_packets = Vec::new();
        while let Some(decoded_packet) = codec.decode(&mut buffer).unwrap() {
            decoded_packets.push(decoded_packet);
        }

        assert_eq!(decoded_packets.len(), 1);
        assert_eq!(decoded_packets[0].as_bytes(), packet.as_bytes());
    }

    #[test]
    fn encode_a_packat_that_is_too_large() {
        let mut codec = MultiIpPacketCodec::new();
        let mut buffer = BytesMut::new();

        let packet = IprPacket::from(Bytes::from_static(
            &[0u8; MAX_PACKET_SIZE + MAX_PACKET_SIZE],
        ));
        codec.encode(packet, &mut buffer).unwrap();
        assert_eq!(
            buffer.len(),
            MAX_PACKET_SIZE + MAX_PACKET_SIZE + LENGTH_PREFIX_SIZE
        );
        codec.encode(IprPacket::Flush, &mut buffer).unwrap();
        assert_eq!(
            buffer.len(),
            MAX_PACKET_SIZE + MAX_PACKET_SIZE + LENGTH_PREFIX_SIZE
        );
    }

    #[test]
    fn check_that_max_size_does_not_flush() {
        let mut codec = MultiIpPacketCodec::new();
        let mut buffer = BytesMut::new();

        let packet = IprPacket::from(Bytes::from_static(&[0u8; MAX_PACKET_SIZE - 2]));
        codec.encode(packet.clone(), &mut buffer).unwrap();
        assert_eq!(buffer.len(), 0);

        let packet = IprPacket::from(Bytes::from_static(&[0u8; MAX_PACKET_SIZE - 2]));
        codec.encode(packet.clone(), &mut buffer).unwrap();
        assert_eq!(buffer.len(), MAX_PACKET_SIZE);
    }
}
