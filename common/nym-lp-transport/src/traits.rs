// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::LpTransportError;
use nym_kkt_ciphersuite::KEM;
use nym_kkt_context::KKTMode;
use nym_lp_packet::{EncryptedLpPacket, OuterHeader};
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;

#[cfg(feature = "io-mocks")]
use nym_test_utils::mocks::async_read_write::MockIOStream;

pub const MAX_TRANSPORT_PACKET_SIZE: usize = 65536; // 64KB max
pub const MAX_HANDSHAKE_PACKET_SIZE: usize = 524287; // 524'160 for mceliece key + a bit of overhead for safety

/// Simple trait allowing sending bytes across.
/// It is not concerned with encryption. It is up to the caller.
// only used in internal code (and tests)
#[allow(async_fn_in_trait)]
pub trait LpHandshakeChannel: Sized {
    /// Write all provided data and immediately flush the buffer
    async fn write_all_and_flush(&mut self, data: &[u8]) -> Result<(), LpTransportError>;

    /// Wrapper around `ReadExact` to return the `Vec<u8>` of `n` bytes directly
    async fn read_n_bytes(&mut self, n: usize) -> Result<Vec<u8>, LpTransportError>;

    /// Send the provided handshake message on the connection
    async fn send_handshake_message<M: HandshakeMessage>(
        &mut self,
        message: M,
        _: KEM,
    ) -> Result<(), LpTransportError> {
        self.write_all_and_flush(&message.into_bytes()).await
    }

    /// Attempt to receive a handshake message of the provided type from the stream
    async fn receive_handshake_message<M: HandshakeMessage>(
        &mut self,
        expected_size: usize,
    ) -> Result<M, LpTransportError> {
        let bytes = self.read_n_bytes(expected_size).await?;
        M::try_from_bytes(bytes)
    }
}

pub trait HandshakeMessage: Sized {
    /// Convert this message into bytes
    fn into_bytes(self) -> Vec<u8>;

    /// Attempt to recover this message from the byte stream
    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, LpTransportError>;

    /// Expected size of this message based on the provided parameters
    fn expected_size(mode: KKTMode, expected_kem: KEM, payload_size: usize) -> usize;

    /// Expected size of the response from the remote party.
    /// `None` if this is the final (PSQ msg2) message of the exchange
    fn response_size(&self, expected_kem: KEM, payload_size: usize) -> Option<usize>;
}

async fn write_all_and_flush_async_write<W>(
    writer: &mut W,
    data: &[u8],
) -> Result<(), LpTransportError>
where
    W: AsyncWrite + Unpin,
{
    writer
        .write_all(data)
        .await
        .map_err(LpTransportError::send_failure)?;
    writer.flush().await.map_err(LpTransportError::send_failure)
}

async fn read_n_bytes_async_read<R>(reader: &mut R, n: usize) -> Result<Vec<u8>, LpTransportError>
where
    R: AsyncRead + Unpin,
{
    let mut buf = vec![0u8; n];
    if n > MAX_HANDSHAKE_PACKET_SIZE {
        return Err(LpTransportError::PacketTooBig { size: n });
    }
    reader
        .read_exact(&mut buf)
        .await
        .map_err(LpTransportError::receive_failure)?;
    Ok(buf)
}

// only used in internal code (and tests)
#[allow(async_fn_in_trait)]
pub trait LpTransportChannel: Sized {
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
    async fn send_length_prefixed_transport_packet(
        &mut self,
        packet: &EncryptedLpPacket,
    ) -> Result<(), LpTransportError>;

    /// Receives an LP packet from a TCP stream with length-prefixed framing without additional parsing
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Errors
    /// Returns an error on network transmission fails.
    async fn receive_length_prefixed_transport_bytes(
        &mut self,
    ) -> Result<Vec<u8>, LpTransportError>;

    /// Receives an LP packet from a TCP stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Errors
    /// Returns an error on network transmission fails.
    async fn receive_length_prefixed_transport_packet(
        &mut self,
    ) -> Result<EncryptedLpPacket, LpTransportError> {
        let mut bytes = self.receive_length_prefixed_transport_bytes().await?;

        if bytes.len() < OuterHeader::SIZE {
            return Err(LpTransportError::PacketTooSmall { size: bytes.len() });
        }

        // split it into the outer header and ciphertext
        let ciphertext = bytes.split_off(OuterHeader::SIZE);

        // SAFETY: we just checked we have at least OuterHeader::SIZE bytes
        #[allow(clippy::unwrap_used)]
        let outer_header = OuterHeader::parse(&bytes).unwrap();

        tracing::trace!(
            "Received LP packet ({} bytes + 4 byte length-prefix)",
            bytes.len()
        );
        Ok(EncryptedLpPacket::new(outer_header, ciphertext))
    }
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
        .write_all(&len.to_le_bytes())
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

async fn receive_length_prefixed_bytes_async_read<R>(
    reader: &mut R,
) -> Result<Vec<u8>, LpTransportError>
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

    let size = u32::from_le_bytes(len_buf) as usize;

    // Sanity check to prevent huge allocations
    if size > MAX_TRANSPORT_PACKET_SIZE {
        return Err(LpTransportError::PacketTooBig { size });
    }

    // Read the actual packet data
    let mut packet_buf = vec![0u8; size];
    reader
        .read_exact(&mut packet_buf)
        .await
        .inspect_err(|e| debug!("Failed to read packet data: {e}"))
        .map_err(LpTransportError::receive_failure)?;

    Ok(packet_buf)
}

impl LpTransportChannel for TcpStream {
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

    async fn send_length_prefixed_transport_packet(
        &mut self,
        packet: &EncryptedLpPacket,
    ) -> Result<(), LpTransportError> {
        send_serialised_packet_async_write(self, packet).await
    }

    async fn receive_length_prefixed_transport_bytes(
        &mut self,
    ) -> Result<Vec<u8>, LpTransportError> {
        receive_length_prefixed_bytes_async_read(self).await
    }
}

#[cfg(feature = "io-mocks")]
impl LpTransportChannel for MockIOStream {
    async fn connect(_endpoint: SocketAddr) -> Result<Self, LpTransportError> {
        Ok(MockIOStream::default())
    }

    fn set_no_delay(&mut self, _nodelay: bool) -> Result<(), LpTransportError> {
        Ok(())
    }

    async fn send_length_prefixed_transport_packet(
        &mut self,
        packet: &EncryptedLpPacket,
    ) -> Result<(), LpTransportError> {
        send_serialised_packet_async_write(self, packet).await
    }

    async fn receive_length_prefixed_transport_bytes(
        &mut self,
    ) -> Result<Vec<u8>, LpTransportError> {
        receive_length_prefixed_bytes_async_read(self).await
    }
}

#[cfg(feature = "io-mocks")]
impl LpHandshakeChannel for MockIOStream {
    async fn write_all_and_flush(&mut self, data: &[u8]) -> Result<(), LpTransportError> {
        write_all_and_flush_async_write(self, data).await
    }

    async fn read_n_bytes(&mut self, n: usize) -> Result<Vec<u8>, LpTransportError> {
        read_n_bytes_async_read(self, n).await
    }
}

impl LpHandshakeChannel for TcpStream {
    async fn write_all_and_flush(&mut self, data: &[u8]) -> Result<(), LpTransportError> {
        write_all_and_flush_async_write(self, data).await
    }

    async fn read_n_bytes(&mut self, n: usize) -> Result<Vec<u8>, LpTransportError> {
        read_n_bytes_async_read(self, n).await
    }
}
