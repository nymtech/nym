use std::time::Duration;

use bytes::{Buf, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] std::io::Error),
}

pub const BUFFER_TIMEOUT: Duration = Duration::from_millis(20);

// Tokio codec for bundling multiple IP packets into one buffer that is at most 1500 bytes long.
// These packets are separated by a 2 byte length prefix. We need a timer so that we don't wait too
// long for the buffer to fill up, since this kills latency.
pub struct MultiIpPacketCodec {
    buffer: BytesMut,
    buffer_timeout: tokio::time::Interval,
}

impl MultiIpPacketCodec {
    pub fn new(buffer_timeout: Duration) -> Self {
        MultiIpPacketCodec {
            buffer: BytesMut::new(),
            buffer_timeout: tokio::time::interval(buffer_timeout),
        }
    }

    // Append a packet to the buffer and return the buffer if it's full
    pub fn append_packet(&mut self, packet: Bytes) -> Option<Bytes> {
        let mut bundled_packets = BytesMut::new();
        self.encode(packet, &mut bundled_packets).unwrap();
        if bundled_packets.is_empty() {
            None
        } else {
            // log::info!("Sphinx packet utilization: {:.2}", self.buffer.len() as f64 / 1500.0);
            Some(bundled_packets.freeze())
        }
    }

    // Flush the current buffer and return it.
    fn flush_current_buffer(&mut self) -> Bytes {
        let mut output_buffer = BytesMut::new();
        std::mem::swap(&mut output_buffer, &mut self.buffer);
        output_buffer.freeze()
    }

    // Wait for the buffer_timeout to tick and then flush the buffer.
    // This is useful when we want to send the buffer even if it's not full.
    pub async fn buffer_timeout(&mut self) -> Option<Bytes> {
        // Wait for buffer_timeout to tick
        let _ = self.buffer_timeout.tick().await;

        // Flush the buffer and return it
        let packets = self.flush_current_buffer();
        if packets.is_empty() {
            None
        } else {
            Some(packets)
        }
    }
}

impl Encoder<Bytes> for MultiIpPacketCodec {
    type Error = Error;

    fn encode(&mut self, packet: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if self.buffer.is_empty() {
            self.buffer_timeout.reset();
        }
        let packet_size = packet.len();

        if self.buffer.len() + packet_size + 2 > 1500 {
            // If the packet doesn't fit in the buffer, send the buffer and then add it to the buffer
            dst.extend_from_slice(&self.buffer);
            self.buffer = BytesMut::new();
            self.buffer_timeout.reset();
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
    type Item = Bytes;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 2 {
            // Not enough bytes to read the length prefix
            return Ok(None);
        }

        let packet_size = u16::from_be_bytes([src[0], src[1]]) as usize;

        if src.len() < packet_size + 2 {
            // Not enough bytes to read the packet
            return Ok(None);
        }

        // Remove the length prefix
        src.advance(2);

        // Read the packet
        let packet = src.split_to(packet_size);

        Ok(Some(packet.freeze()))
    }
}
