// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::LpTransportError;
use nym_lp_packet::{EncryptedLpPacket, OuterHeader};
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;

#[cfg(feature = "io-mocks")]
use nym_test_utils::mocks::async_read_write::MockIOStream;

pub const MAX_PACKET_SIZE: usize = 65536; // 64KB max

/// Simple trait allowing sending bytes across.
/// It is not concerned with encryption. It is up to the caller.
// only used in internal code (and tests)
#[allow(async_fn_in_trait)]
pub trait LpChannel: AsyncRead + AsyncWrite + Sized {
    /// Write all provided data and immediately flush the buffer
    async fn write_all_and_flush(&mut self, data: &[u8]) -> Result<(), LpTransportError>
    where
        Self: Unpin,
    {
        self.write_all(data)
            .await
            .map_err(LpTransportError::send_failure)?;
        self.flush().await.map_err(LpTransportError::send_failure)
    }

    /// Wrapper around `ReadExact` to return the `Vec<u8>` of `n` bytes directly
    async fn read_n_bytes(&mut self, n: usize) -> Result<Vec<u8>, LpTransportError>
    where
        Self: Unpin,
    {
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf)
            .await
            .map_err(LpTransportError::receive_failure)?;
        Ok(buf)
    }
}

// only used in internal code (and tests)
#[allow(async_fn_in_trait)]
pub trait LpTransport: Sized {
    async fn connect(endpoint: SocketAddr) -> Result<Self, LpTransportError>;

    fn set_no_delay(&mut self, nodelay: bool) -> Result<(), LpTransportError>;

    /// Sends a serialised and encrypted LP packet over the data stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Arguments
    /// * `packet` - The encrypted LP packet to send
    ///
    /// # Errors
    /// Returns an error on network transmission fails.
    async fn send_length_prefixed_packet(
        &mut self,
        packet: &EncryptedLpPacket,
    ) -> Result<(), LpTransportError>;

    /// Receives an LP packet from a TCP stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Errors
    /// Returns an error on network transmission fails.
    async fn receive_length_prefixed_packet(
        &mut self,
    ) -> Result<EncryptedLpPacket, LpTransportError>;
}

async fn send_serialised_packet_async_write<W>(
    writer: &mut W,
    packet: &EncryptedLpPacket,
) -> Result<(), LpTransportError>
where
    W: AsyncWrite + Unpin,
{
    // Send 4-byte length prefix (u32 big-endian)
    let len = packet.encoded_length() as u32;
    writer
        .write_all(&len.to_be_bytes())
        .await
        .inspect_err(|e| debug!("Failed to send packet length: {e}"))
        .map_err(LpTransportError::send_failure)?;

    // TODO: benchmark whether it'd be faster to concatenate all slices slices and
    // use a single `write_all` call

    // Send the outer header
    writer
        .write_all(&packet.outer_header().to_bytes())
        .await
        .inspect_err(|e| debug!("Failed to send packet data: {e}"))
        .map_err(LpTransportError::send_failure)?;

    // Send the actual packet data
    writer
        .write_all(packet.ciphertext())
        .await
        .inspect_err(|e| debug!("Failed to send packet data: {e}"))
        .map_err(LpTransportError::send_failure)?;

    // Flush to ensure data is sent immediately
    writer
        .flush()
        .await
        .inspect_err(|e| debug!("Failed to flush stream: {e}"))
        .map_err(LpTransportError::send_failure)?;

    tracing::trace!(
        "Sent LP packet ({} bytes + 4 byte length-prefix)",
        packet.encoded_length()
    );
    Ok(())
}

async fn receive_raw_packet_async_read<R>(
    reader: &mut R,
) -> Result<EncryptedLpPacket, LpTransportError>
where
    R: AsyncRead + Unpin,
{
    // Read 4-byte length prefix (u32 big-endian)
    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .inspect_err(|e| debug!("Failed to read packet length: {e}"))
        .map_err(LpTransportError::receive_failure)?;

    let size = u32::from_be_bytes(len_buf) as usize;

    // Sanity check to prevent huge allocations
    if size > MAX_PACKET_SIZE {
        return Err(LpTransportError::PacketTooBig { size });
    }

    if size < OuterHeader::SIZE {
        return Err(LpTransportError::PacketTooSmall { size });
    }

    // Read the actual packet data
    let mut packet_buf = vec![0u8; size];
    reader
        .read_exact(&mut packet_buf)
        .await
        .inspect_err(|e| debug!("Failed to read packet data: {e}"))
        .map_err(LpTransportError::receive_failure)?;

    // split it into the outer header and ciphertext
    let ciphertext = packet_buf.split_off(OuterHeader::SIZE);

    // SAFETY: we just checked we have at least OuterHeader::SIZE bytes
    #[allow(clippy::unwrap_used)]
    let outer_header = OuterHeader::parse(&packet_buf).unwrap();

    tracing::trace!("Sent LP packet ({size} bytes + 4 byte length-prefix)",);
    Ok(EncryptedLpPacket::new(outer_header, ciphertext))
}

impl LpTransport for TcpStream {
    async fn connect(endpoint: SocketAddr) -> Result<Self, LpTransportError> {
        TcpStream::connect(endpoint)
            .await
            .map_err(|err| LpTransportError::connection_failure(err.to_string()))
    }

    fn set_no_delay(&mut self, nodelay: bool) -> Result<(), LpTransportError> {
        // Set TCP_NODELAY for low latency
        self.set_nodelay(nodelay)
            .map_err(|err| LpTransportError::connection_config(err.to_string()))
    }

    async fn send_length_prefixed_packet(
        &mut self,
        packet: &EncryptedLpPacket,
    ) -> Result<(), LpTransportError> {
        send_serialised_packet_async_write(self, packet).await
    }

    async fn receive_length_prefixed_packet(
        &mut self,
    ) -> Result<EncryptedLpPacket, LpTransportError> {
        receive_raw_packet_async_read(self).await
    }
}

#[cfg(feature = "io-mocks")]
impl LpTransport for MockIOStream {
    async fn connect(_endpoint: SocketAddr) -> Result<Self, LpTransportError> {
        Ok(MockIOStream::default())
    }

    fn set_no_delay(&mut self, _nodelay: bool) -> Result<(), LpTransportError> {
        Ok(())
    }

    async fn send_length_prefixed_packet(
        &mut self,
        packet: &EncryptedLpPacket,
    ) -> Result<(), LpTransportError> {
        send_serialised_packet_async_write(self, packet).await
    }

    async fn receive_length_prefixed_packet(
        &mut self,
    ) -> Result<EncryptedLpPacket, LpTransportError> {
        receive_raw_packet_async_read(self).await
    }
}

#[cfg(feature = "io-mocks")]
impl LpChannel for MockIOStream {}
impl LpChannel for TcpStream {}
