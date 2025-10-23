// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP transport layer for handling post-handshake communication.
//!
//! The transport layer manages data flow after a successful Noise protocol handshake,
//! handling encryption, decryption, and reliable message delivery over the LP connection.

use super::error::{LpClientError, Result};
use bytes::BytesMut;
use nym_lp::codec::{parse_lp_packet, serialize_lp_packet};
use nym_lp::state_machine::{LpAction, LpInput, LpStateBare, LpStateMachine};
use nym_lp::LpPacket;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Handles LP transport after successful handshake.
///
/// This struct manages encrypted data transmission using an established LP session,
/// providing methods for sending and receiving arbitrary data over the secure channel.
///
/// # Usage
/// ```ignore
/// // After handshake and registration
/// let transport = client.into_transport()?;
///
/// // Send arbitrary data
/// transport.send_data(b"hello").await?;
///
/// // Receive data
/// let response = transport.receive_data().await?;
///
/// // Close when done
/// transport.close().await?;
/// ```
pub struct LpTransport {
    /// TCP stream for network I/O
    stream: TcpStream,

    /// LP state machine managing encryption/decryption
    state_machine: LpStateMachine,
}

impl LpTransport {
    /// Creates a new LP transport handler from an established connection.
    ///
    /// This should be called after a successful Noise protocol handshake.
    /// The state machine must be in Transport state.
    ///
    /// # Arguments
    /// * `stream` - The TCP stream connected to the gateway
    /// * `state_machine` - The LP state machine in Transport state
    ///
    /// # Errors
    /// Returns an error if the state machine is not in Transport state.
    pub fn from_handshake(stream: TcpStream, state_machine: LpStateMachine) -> Result<Self> {
        // Validate that handshake is complete
        match state_machine.bare_state() {
            LpStateBare::Transport => Ok(Self {
                stream,
                state_machine,
            }),
            other => Err(LpClientError::Transport(format!(
                "Cannot create transport: state machine is in {:?} state, expected Transport",
                other
            ))),
        }
    }

    /// Sends arbitrary encrypted data over the LP connection.
    ///
    /// The data is encrypted using the established LP session and sent with
    /// length-prefixed framing (4-byte big-endian u32 length + packet data).
    ///
    /// # Arguments
    /// * `data` - The plaintext data to send
    ///
    /// # Errors
    /// Returns an error if:
    /// - Encryption fails
    /// - Network transmission fails
    /// - State machine returns unexpected action
    pub async fn send_data(&mut self, data: &[u8]) -> Result<()> {
        tracing::trace!("Sending {} bytes over LP transport", data.len());

        // Encrypt via state machine
        let action = self
            .state_machine
            .process_input(LpInput::SendData(data.to_vec()))
            .ok_or_else(|| {
                LpClientError::Transport("State machine returned no action for SendData".to_string())
            })?
            .map_err(|e| LpClientError::Transport(format!("Failed to encrypt data: {}", e)))?;

        // Extract and send packet
        match action {
            LpAction::SendPacket(packet) => {
                self.send_packet(&packet).await?;
                tracing::trace!("Successfully sent encrypted data packet");
                Ok(())
            }
            other => Err(LpClientError::Transport(format!(
                "Unexpected action when sending data: {:?}",
                other
            ))),
        }
    }

    /// Receives and decrypts data from the LP connection.
    ///
    /// Reads a length-prefixed packet, decrypts it using the LP session,
    /// and returns the plaintext data.
    ///
    /// # Returns
    /// The decrypted plaintext data as a Vec<u8>
    ///
    /// # Errors
    /// Returns an error if:
    /// - Network reception fails
    /// - Packet parsing fails
    /// - Decryption fails
    /// - State machine returns unexpected action
    pub async fn receive_data(&mut self) -> Result<Vec<u8>> {
        tracing::trace!("Waiting to receive data over LP transport");

        // Receive packet from network
        let packet = self.receive_packet().await?;

        // Decrypt via state machine
        let action = self
            .state_machine
            .process_input(LpInput::ReceivePacket(packet))
            .ok_or_else(|| {
                LpClientError::Transport(
                    "State machine returned no action for ReceivePacket".to_string(),
                )
            })?
            .map_err(|e| LpClientError::Transport(format!("Failed to decrypt data: {}", e)))?;

        // Extract decrypted data
        match action {
            LpAction::DeliverData(data) => {
                tracing::trace!("Successfully received and decrypted {} bytes", data.len());
                Ok(data.to_vec())
            }
            other => Err(LpClientError::Transport(format!(
                "Unexpected action when receiving data: {:?}",
                other
            ))),
        }
    }

    /// Gracefully closes the LP connection.
    ///
    /// Sends a close signal to the peer and shuts down the TCP stream.
    ///
    /// # Errors
    /// Returns an error if the close operation fails.
    pub async fn close(mut self) -> Result<()> {
        tracing::debug!("Closing LP transport");

        // Signal close to state machine
        if let Some(action_result) = self.state_machine.process_input(LpInput::Close) {
            match action_result {
                Ok(LpAction::ConnectionClosed) => {
                    tracing::debug!("LP connection closed by state machine");
                }
                Ok(other) => {
                    tracing::warn!(
                        "Unexpected action when closing connection: {:?}",
                        other
                    );
                }
                Err(e) => {
                    tracing::warn!("Error closing LP connection: {}", e);
                }
            }
        }

        // Shutdown TCP stream
        if let Err(e) = self.stream.shutdown().await {
            tracing::warn!("Error shutting down TCP stream: {}", e);
        }

        tracing::info!("LP transport closed");
        Ok(())
    }

    /// Checks if the transport is in a valid state for data transfer.
    ///
    /// Returns true if the state machine is in Transport state.
    pub fn is_connected(&self) -> bool {
        matches!(self.state_machine.bare_state(), LpStateBare::Transport)
    }

    /// Sends an LP packet over the TCP stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    async fn send_packet(&mut self, packet: &LpPacket) -> Result<()> {
        // Serialize the packet
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(packet, &mut packet_buf)
            .map_err(|e| LpClientError::Transport(format!("Failed to serialize packet: {}", e)))?;

        // Send 4-byte length prefix (u32 big-endian)
        let len = packet_buf.len() as u32;
        self.stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to send packet length: {}", e)))?;

        // Send the actual packet data
        self.stream
            .write_all(&packet_buf)
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to send packet data: {}", e)))?;

        // Flush to ensure data is sent immediately
        self.stream
            .flush()
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to flush stream: {}", e)))?;

        tracing::trace!("Sent LP packet ({} bytes + 4 byte header)", packet_buf.len());
        Ok(())
    }

    /// Receives an LP packet from the TCP stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    async fn receive_packet(&mut self) -> Result<LpPacket> {
        // Read 4-byte length prefix (u32 big-endian)
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to read packet length: {}", e)))?;

        let packet_len = u32::from_be_bytes(len_buf) as usize;

        // Sanity check to prevent huge allocations
        const MAX_PACKET_SIZE: usize = 65536; // 64KB max
        if packet_len > MAX_PACKET_SIZE {
            return Err(LpClientError::Transport(format!(
                "Packet size {} exceeds maximum {}",
                packet_len, MAX_PACKET_SIZE
            )));
        }

        // Read the actual packet data
        let mut packet_buf = vec![0u8; packet_len];
        self.stream
            .read_exact(&mut packet_buf)
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to read packet data: {}", e)))?;

        // Parse the packet
        let packet = parse_lp_packet(&packet_buf)
            .map_err(|e| LpClientError::Transport(format!("Failed to parse packet: {}", e)))?;

        tracing::trace!(
            "Received LP packet ({} bytes + 4 byte header)",
            packet_len
        );
        Ok(packet)
    }
}
