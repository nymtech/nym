// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::handshake::LpGatewayHandshake;
use super::messages::{LpRegistrationRequest, LpRegistrationResponse};
use super::registration::process_registration;
use super::LpHandlerState;
use crate::error::GatewayError;
use nym_lp::{
    keypair::{Keypair, PublicKey},
    LpMessage, LpPacket, LpSession,
};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::*;

pub struct LpConnectionHandler {
    stream: TcpStream,
    remote_addr: SocketAddr,
    state: LpHandlerState,
}

impl LpConnectionHandler {
    pub fn new(stream: TcpStream, remote_addr: SocketAddr, state: LpHandlerState) -> Self {
        Self {
            stream,
            remote_addr,
            state,
        }
    }

    pub async fn handle(mut self) -> Result<(), GatewayError> {
        debug!("Handling LP connection from {}", self.remote_addr);

        // For LP, we need:
        // 1. Gateway's keypair (from local_identity)
        // 2. Client's public key (will be received during handshake)
        // 3. PSK (pre-shared key) - for now use a placeholder

        // Generate fresh LP keypair (x25519) for this connection
        // Using Keypair::default() which generates a new random x25519 keypair
        // This is secure and simple - each connection gets its own keypair
        let gateway_keypair = Keypair::default();

        // Receive client's public key via ClientHello message
        // The client initiates by sending ClientHello as first packet
        let client_pubkey = self.receive_client_hello().await?;

        // Generate or retrieve PSK for this session
        // TODO(nym-16): Implement proper PSK management
        // Temporary solution: use gateway's identity public key as PSK
        let psk = self.state.local_identity.public_key().to_bytes();

        // Create LP handshake as responder
        let handshake = LpGatewayHandshake::new_responder(
            &gateway_keypair,
            &client_pubkey,
            &psk,
        )?;

        // Complete the LP handshake
        let session = handshake.complete(&mut self.stream).await?;

        info!("LP handshake completed for {} (session {})",
              self.remote_addr, session.id());

        // After handshake, receive registration request
        let request = self.receive_registration_request(&session).await?;

        debug!("LP registration request from {}: mode={:?}",
               self.remote_addr, request.mode);

        // Process registration (verify credentials, add peer, etc.)
        let response = process_registration(request, &self.state).await;

        // Send response
        if let Err(e) = self.send_registration_response(&session, response.clone()).await {
            warn!("Failed to send LP response to {}: {}", self.remote_addr, e);
            return Err(e);
        }

        if response.success {
            info!("LP registration successful for {} (session {})",
                  self.remote_addr, response.session_id);
        } else {
            warn!("LP registration failed for {}: {:?}",
                  self.remote_addr, response.error);
        }

        Ok(())
    }

    /// Receive client's public key via ClientHello message
    async fn receive_client_hello(&mut self) -> Result<PublicKey, GatewayError> {
        // Receive first packet which should be ClientHello
        let packet = self.receive_lp_packet().await?;

        // Verify it's a ClientHello message
        match packet.message() {
            LpMessage::ClientHello(hello_data) => {
                // Validate protocol version (currently only v1)
                if hello_data.protocol_version != 1 {
                    return Err(GatewayError::LpProtocolError(
                        format!("Unsupported protocol version: {}", hello_data.protocol_version)
                    ));
                }

                // Convert bytes to PublicKey
                PublicKey::from_bytes(&hello_data.client_lp_public_key)
                    .map_err(|e| GatewayError::LpProtocolError(
                        format!("Invalid client public key: {}", e)
                    ))
            }
            other => {
                Err(GatewayError::LpProtocolError(
                    format!("Expected ClientHello, got {}", other)
                ))
            }
        }
    }

    /// Receive registration request after handshake
    async fn receive_registration_request(
        &mut self,
        session: &LpSession,
    ) -> Result<LpRegistrationRequest, GatewayError> {
        // Read LP packet containing the registration request
        let packet = self.receive_lp_packet().await?;

        // Verify it's from the correct session
        if packet.header().session_id != session.id() {
            return Err(GatewayError::LpProtocolError(
                format!("Session ID mismatch: expected {}, got {}",
                        session.id(), packet.header().session_id)
            ));
        }

        // Extract registration request from LP message
        match packet.message() {
            LpMessage::EncryptedData(data) => {
                // Deserialize registration request
                bincode::deserialize(&data)
                    .map_err(|e| GatewayError::LpProtocolError(
                        format!("Failed to deserialize registration request: {}", e)
                    ))
            }
            other => {
                Err(GatewayError::LpProtocolError(
                    format!("Expected EncryptedData message, got {:?}", other)
                ))
            }
        }
    }

    /// Send registration response after processing
    async fn send_registration_response(
        &mut self,
        session: &LpSession,
        response: LpRegistrationResponse,
    ) -> Result<(), GatewayError> {
        // Serialize response
        let data = bincode::serialize(&response)
            .map_err(|e| GatewayError::LpProtocolError(
                format!("Failed to serialize response: {}", e)
            ))?;

        // Create LP packet with response
        let packet = session.create_data_packet(data)
            .map_err(|e| GatewayError::LpProtocolError(
                format!("Failed to create data packet: {}", e)
            ))?;

        // Send the packet
        self.send_lp_packet(&packet).await
    }

    /// Receive an LP packet from the stream with proper length-prefixed framing
    async fn receive_lp_packet(&mut self) -> Result<LpPacket, GatewayError> {
        use nym_lp::codec::parse_lp_packet;

        // Read 4-byte length prefix (u32 big-endian)
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to read packet length: {}", e)
            ))?;

        let packet_len = u32::from_be_bytes(len_buf) as usize;

        // Sanity check to prevent huge allocations
        const MAX_PACKET_SIZE: usize = 65536; // 64KB max
        if packet_len > MAX_PACKET_SIZE {
            return Err(GatewayError::LpProtocolError(
                format!("Packet size {} exceeds maximum {}", packet_len, MAX_PACKET_SIZE)
            ));
        }

        // Read the actual packet data
        let mut packet_buf = vec![0u8; packet_len];
        self.stream.read_exact(&mut packet_buf).await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to read packet data: {}", e)
            ))?;

        parse_lp_packet(&packet_buf)
            .map_err(|e| GatewayError::LpProtocolError(
                format!("Failed to parse LP packet: {}", e)
            ))
    }

    /// Send an LP packet over the stream with proper length-prefixed framing
    async fn send_lp_packet(&mut self, packet: &LpPacket) -> Result<(), GatewayError> {
        use nym_lp::codec::serialize_lp_packet;
        use bytes::BytesMut;

        // Serialize the packet first
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(packet, &mut packet_buf)
            .map_err(|e| GatewayError::LpProtocolError(
                format!("Failed to serialize packet: {}", e)
            ))?;

        // Send 4-byte length prefix (u32 big-endian)
        let len = packet_buf.len() as u32;
        self.stream.write_all(&len.to_be_bytes()).await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to send packet length: {}", e)
            ))?;

        // Send the actual packet data
        self.stream.write_all(&packet_buf).await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to send packet data: {}", e)
            ))?;

        self.stream.flush().await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to flush stream: {}", e)
            ))?;

        Ok(())
    }
}

// Extension trait for LpSession to create packets
// This would ideally be part of nym-lp
trait LpSessionExt {
    fn create_data_packet(&self, data: Vec<u8>) -> Result<LpPacket, nym_lp::LpError>;
}

impl LpSessionExt for LpSession {
    fn create_data_packet(&self, data: Vec<u8>) -> Result<LpPacket, nym_lp::LpError> {
        use nym_lp::packet::LpHeader;

        let header = LpHeader {
            protocol_version: 1,
            session_id: self.id(),
            counter: 0, // TODO: Use actual counter from session
        };

        let message = LpMessage::EncryptedData(data);

        Ok(LpPacket::new(header, message))
    }
}