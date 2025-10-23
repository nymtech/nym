// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::GatewayError;
use nym_lp::{
    keypair::{Keypair, PublicKey},
    state_machine::{LpAction, LpInput, LpStateMachine},
    LpPacket, LpSession,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::*;

/// Wrapper around the nym-lp state machine for gateway-side LP connections
pub struct LpGatewayHandshake {
    state_machine: LpStateMachine,
}

impl LpGatewayHandshake {
    /// Create a new responder (gateway side) handshake
    pub fn new_responder(
        local_keypair: &Keypair,
        remote_public_key: &PublicKey,
        psk: &[u8; 32],
    ) -> Result<Self, GatewayError> {
        let state_machine = LpStateMachine::new(
            false, // responder
            local_keypair,
            remote_public_key,
            psk,
        )
        .map_err(|e| {
            GatewayError::LpHandshakeError(format!("Failed to create state machine: {}", e))
        })?;

        Ok(Self { state_machine })
    }

    /// Complete the handshake and return the established session
    pub async fn complete(mut self, stream: &mut TcpStream) -> Result<LpSession, GatewayError> {
        debug!("Starting LP handshake as responder");

        // Start the handshake
        if let Some(action) = self.state_machine.process_input(LpInput::StartHandshake) {
            match action {
                Ok(LpAction::SendPacket(packet)) => {
                    self.send_packet(stream, &packet).await?;
                }
                Ok(_) => {
                    // Unexpected action at this stage
                    return Err(GatewayError::LpHandshakeError(
                        "Unexpected action at handshake start".to_string(),
                    ));
                }
                Err(e) => {
                    return Err(GatewayError::LpHandshakeError(format!(
                        "Failed to start handshake: {}",
                        e
                    )));
                }
            }
        }

        // Continue handshake until complete
        loop {
            // Read incoming packet
            let packet = self.receive_packet(stream).await?;

            // Process the received packet
            if let Some(action) = self
                .state_machine
                .process_input(LpInput::ReceivePacket(packet))
            {
                match action {
                    Ok(LpAction::SendPacket(response_packet)) => {
                        self.send_packet(stream, &response_packet).await?;
                    }
                    Ok(LpAction::HandshakeComplete) => {
                        info!("LP handshake completed successfully");
                        break;
                    }
                    Ok(other) => {
                        debug!("Received action during handshake: {:?}", other);
                    }
                    Err(e) => {
                        return Err(GatewayError::LpHandshakeError(format!(
                            "Handshake error: {}",
                            e
                        )));
                    }
                }
            }
        }

        // Extract the session from the state machine
        self.state_machine.into_session().map_err(|e| {
            GatewayError::LpHandshakeError(format!("Failed to get session after handshake: {}", e))
        })
    }

    /// Send an LP packet over the stream with proper length-prefixed framing
    async fn send_packet(
        &self,
        stream: &mut TcpStream,
        packet: &LpPacket,
    ) -> Result<(), GatewayError> {
        use bytes::BytesMut;
        use nym_lp::codec::serialize_lp_packet;

        // Serialize the packet first
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(packet, &mut packet_buf).map_err(|e| {
            GatewayError::LpProtocolError(format!("Failed to serialize packet: {}", e))
        })?;

        // Send 4-byte length prefix (u32 big-endian)
        let len = packet_buf.len() as u32;
        stream.write_all(&len.to_be_bytes()).await.map_err(|e| {
            GatewayError::LpConnectionError(format!("Failed to send packet length: {}", e))
        })?;

        // Send the actual packet data
        stream.write_all(&packet_buf).await.map_err(|e| {
            GatewayError::LpConnectionError(format!("Failed to send packet data: {}", e))
        })?;

        stream.flush().await.map_err(|e| {
            GatewayError::LpConnectionError(format!("Failed to flush stream: {}", e))
        })?;

        debug!(
            "Sent LP packet ({} bytes + 4 byte header)",
            packet_buf.len()
        );
        Ok(())
    }

    /// Receive an LP packet from the stream with proper length-prefixed framing
    async fn receive_packet(&self, stream: &mut TcpStream) -> Result<LpPacket, GatewayError> {
        use nym_lp::codec::parse_lp_packet;

        // Read 4-byte length prefix (u32 big-endian)
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.map_err(|e| {
            GatewayError::LpConnectionError(format!("Failed to read packet length: {}", e))
        })?;

        let packet_len = u32::from_be_bytes(len_buf) as usize;

        // Sanity check to prevent huge allocations
        const MAX_PACKET_SIZE: usize = 65536; // 64KB max
        if packet_len > MAX_PACKET_SIZE {
            return Err(GatewayError::LpProtocolError(format!(
                "Packet size {} exceeds maximum {}",
                packet_len, MAX_PACKET_SIZE
            )));
        }

        // Read the actual packet data
        let mut packet_buf = vec![0u8; packet_len];
        stream.read_exact(&mut packet_buf).await.map_err(|e| {
            GatewayError::LpConnectionError(format!("Failed to read packet data: {}", e))
        })?;

        let packet = parse_lp_packet(&packet_buf)
            .map_err(|e| GatewayError::LpProtocolError(format!("Failed to parse packet: {}", e)))?;

        debug!("Received LP packet ({} bytes + 4 byte header)", packet_len);
        Ok(packet)
    }
}
