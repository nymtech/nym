// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "io-mocks")]
use nym_test_utils::mocks::async_read_write::MockIOStream;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;

// only used in internal code (and tests)
#[allow(async_fn_in_trait)]
pub trait LpChannel: Sized {
    /// Sends a serialised acket over the data stream.
    ///
    /// # Arguments
    /// * `packet_data` - The serialised packet to send
    ///
    /// # Errors
    /// Returns an error on network transmission fails.
    async fn send_serialised_packet(&mut self, packet_data: &[u8]) -> std::io::Result<()>;

    /// Receives a data chunk of the set length from the data stream.
    ///
    /// # Errors
    /// Returns an error on network transmission fails.
    async fn receive_raw_packet(&mut self, len: usize) -> std::io::Result<Vec<u8>>;
}

// only used in internal code (and tests)
#[allow(async_fn_in_trait)]
pub trait LpTransport: Sized {
    async fn connect(endpoint: SocketAddr) -> std::io::Result<Self>;

    fn set_no_delay(&mut self, nodelay: bool) -> std::io::Result<()>;

    /// Sends a serialised (and optionally encrypted) LP packet over the data stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Arguments
    /// * `packet_data` - The serialised LP packet to send
    ///
    /// # Errors
    /// Returns an error on network transmission fails.
    async fn send_serialised_packet(&mut self, packet_data: &[u8]) -> std::io::Result<()>;

    /// Receives an LP packet from a TCP stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Errors
    /// Returns an error on network transmission fails.
    async fn receive_raw_packet(&mut self) -> std::io::Result<Vec<u8>>;
}

async fn send_serialised_packet_async_write<W>(
    writer: &mut W,
    packet_data: &[u8],
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    // Send 4-byte length prefix (u32 big-endian)
    let len = packet_data.len() as u32;
    writer
        .write_all(&len.to_be_bytes())
        .await
        .inspect_err(|e| debug!("Failed to send packet length: {e}"))?;

    // Send the actual packet data
    writer
        .write_all(packet_data)
        .await
        .inspect_err(|e| debug!("Failed to send packet data: {e}"))?;

    // Flush to ensure data is sent immediately
    writer
        .flush()
        .await
        .inspect_err(|e| debug!("Failed to flush stream: {e}"))?;

    tracing::trace!(
        "Sent LP packet ({} bytes + 4 byte header)",
        packet_data.len()
    );
    Ok(())
}

async fn receive_raw_packet_async_read<R>(reader: &mut R) -> std::io::Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    // Read 4-byte length prefix (u32 big-endian)
    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .inspect_err(|e| debug!("Failed to read packet length: {e}"))?;

    let packet_len = u32::from_be_bytes(len_buf) as usize;

    // Sanity check to prevent huge allocations
    const MAX_PACKET_SIZE: usize = 65536; // 64KB max
    if packet_len > MAX_PACKET_SIZE {
        return Err(std::io::Error::other(format!(
            "Packet size {packet_len} exceeds maximum {MAX_PACKET_SIZE}",
        )));
    }

    // Read the actual packet data
    let mut packet_buf = vec![0u8; packet_len];
    reader
        .read_exact(&mut packet_buf)
        .await
        .inspect_err(|e| debug!("Failed to read packet data: {e}"))?;

    tracing::trace!("Received LP packet ({packet_len} bytes + 4 byte header)");
    Ok(packet_buf)
}

impl LpTransport for TcpStream {
    async fn connect(endpoint: SocketAddr) -> std::io::Result<Self> {
        TcpStream::connect(endpoint).await
    }

    fn set_no_delay(&mut self, nodelay: bool) -> std::io::Result<()> {
        // Set TCP_NODELAY for low latency
        self.set_nodelay(nodelay)
    }

    async fn send_serialised_packet(&mut self, packet_data: &[u8]) -> std::io::Result<()> {
        send_serialised_packet_async_write(self, packet_data).await
    }

    async fn receive_raw_packet(&mut self) -> std::io::Result<Vec<u8>> {
        receive_raw_packet_async_read(self).await
    }
}

#[cfg(feature = "io-mocks")]
impl LpTransport for MockIOStream {
    async fn connect(_endpoint: SocketAddr) -> std::io::Result<Self> {
        Ok(MockIOStream::default())
    }

    fn set_no_delay(&mut self, _nodelay: bool) -> std::io::Result<()> {
        Ok(())
    }

    async fn send_serialised_packet(&mut self, packet_data: &[u8]) -> std::io::Result<()> {
        send_serialised_packet_async_write(self, packet_data).await
    }

    async fn receive_raw_packet(&mut self) -> std::io::Result<Vec<u8>> {
        receive_raw_packet_async_read(self).await
    }
}
