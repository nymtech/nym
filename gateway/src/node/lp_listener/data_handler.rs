// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! LP Data Handler - UDP listener for LP data plane (port 51264)
//!
//! This module handles the data plane for LP clients that have completed registration
//! via the control plane (TCP:41264). LP-wrapped Sphinx packets arrive here, get
//! decrypted, and are forwarded into the mixnet.
//!
//! # Packet Flow
//!
//! ```text
//! LP Client → UDP:51264 → LP Data Handler → Mixnet Entry
//!           LP(Sphinx)      decrypt LP      forward Sphinx
//! ```
//!
//! # Wire Format
//!
//! Each UDP packet is a complete LP packet:
//! - Header (8 bytes): receiver_idx (4) + counter (4)
//! - Payload: Outer AEAD encrypted Sphinx packet
//!
//! The receiver_idx is used to look up the session established during LP registration.

use super::LpHandlerState;
use crate::error::GatewayError;
use nym_lp::state_machine::{LpAction, LpInput};
use nym_metrics::inc;
use nym_sphinx::forwarding::packet::MixPacket;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::*;

/// Maximum UDP packet size we'll accept
/// Sphinx packets are typically ~2KB, LP overhead is ~50 bytes, so 4KB is plenty
const MAX_UDP_PACKET_SIZE: usize = 4096;

/// LP Data Handler for UDP data plane
pub struct LpDataHandler {
    /// UDP socket for receiving LP-wrapped Sphinx packets
    socket: Arc<UdpSocket>,

    /// Shared state with TCP control plane
    state: LpHandlerState,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataHandler {
    /// Create a new LP data handler
    pub async fn new(
        bind_addr: SocketAddr,
        state: LpHandlerState,
        shutdown: nym_task::ShutdownToken,
    ) -> Result<Self, GatewayError> {
        let socket = UdpSocket::bind(bind_addr).await.map_err(|e| {
            error!("Failed to bind LP data socket to {bind_addr}: {e}");
            GatewayError::ListenerBindFailure {
                address: bind_addr.to_string(),
                source: Box::new(e),
            }
        })?;

        info!("LP data handler listening on UDP {bind_addr}");

        Ok(Self {
            socket: Arc::new(socket),
            state,
            shutdown,
        })
    }

    /// Run the UDP packet receive loop
    pub async fn run(self) -> Result<(), GatewayError> {
        let mut buf = vec![0u8; MAX_UDP_PACKET_SIZE];

        loop {
            tokio::select! {
                biased;

                _ = self.shutdown.cancelled() => {
                    info!("LP data handler: received shutdown signal");
                    break;
                }

                result = self.socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, src_addr)) => {
                            // Process packet in place (no spawn - UDP is fast)
                            if let Err(e) = self.handle_packet(&buf[..len], src_addr).await {
                                debug!("LP data packet error from {}: {}", src_addr, e);
                                inc!("lp_data_packet_errors");
                            }
                        }
                        Err(e) => {
                            warn!("LP data socket recv error: {}", e);
                            inc!("lp_data_recv_errors");
                        }
                    }
                }
            }
        }

        info!("LP data handler shutdown complete");
        Ok(())
    }

    /// Handle a single UDP packet
    ///
    /// # Packet Processing Steps
    /// 1. Parse LP header to get receiver_idx (for routing)
    /// 2. Look up session state machine by receiver_idx
    /// 3. Process packet through state machine (handles decryption + replay protection)
    /// 4. Forward decrypted Sphinx packet to mixnet
    ///
    /// # Security
    /// The state machine's `process_input()` method handles replay protection by:
    /// - Checking packet counter against receiving window
    /// - Marking counter as used after successful decryption
    /// This prevents replay attacks where captured packets are re-sent.
    async fn handle_packet(&self, packet: &[u8], src_addr: SocketAddr) -> Result<(), GatewayError> {
        inc!("lp_data_packets_received");

        // Step 1: Parse LP header (always cleartext for routing)
        let header = nym_lp::codec::parse_lp_header_only(packet).map_err(|e| {
            GatewayError::LpProtocolError(format!("Failed to parse LP header: {}", e))
        })?;

        let receiver_idx = header.receiver_idx;
        let counter = header.counter;
        let len = packet.len();

        trace!("LP data packet from {src_addr} (receiver_idx={receiver_idx}, counter={counter}, len={len})");

        // Step 2: Look up session state machine by receiver_idx (mutable for state updates)
        let mut state_entry = self
            .state
            .session_states
            .get_mut(&receiver_idx)
            .ok_or_else(|| {
                inc!("lp_data_unknown_session");
                GatewayError::LpProtocolError(format!(
                    "Unknown session for receiver_idx {receiver_idx}"
                ))
            })?;

        // Update last activity timestamp
        state_entry.value().touch();

        // Step 3: Get outer AEAD key for packet parsing
        let outer_key = state_entry
            .value()
            .state
            .session()
            .map_err(|e| GatewayError::LpProtocolError(format!("Session error: {}", e)))?
            .outer_aead_key()
            .ok_or_else(|| {
                GatewayError::LpProtocolError("Session has no outer AEAD key".to_string())
            })?;

        // Parse full packet with outer AEAD decryption
        let lp_packet = nym_lp::codec::parse_lp_packet(packet, Some(&outer_key)).map_err(|e| {
            inc!("lp_data_decrypt_errors");
            GatewayError::LpProtocolError(format!("Failed to decrypt LP packet: {}", e))
        })?;

        // Step 4: Process packet through state machine
        // This handles:
        // - Replay protection (counter check + mark)
        // - Inner Noise decryption
        // - Subsession handling if applicable
        let state_machine = &mut state_entry.value_mut().state;

        let action = state_machine
            .process_input(LpInput::ReceivePacket(lp_packet))
            .ok_or_else(|| {
                GatewayError::LpProtocolError("State machine returned no action".to_string())
            })?
            .map_err(|e| {
                inc!("lp_data_state_machine_errors");
                GatewayError::LpProtocolError(format!("State machine error: {}", e))
            })?;

        // Release session lock before forwarding
        drop(state_entry);

        // Step 5: Handle the action from state machine
        match action {
            LpAction::DeliverData(data) => {
                // Decrypted application data - forward as Sphinx packet
                self.forward_sphinx_packet(&data).await?;
                inc!("lp_data_packets_forwarded");
                Ok(())
            }
            LpAction::SendPacket(_response_packet) => {
                // UDP is connectionless - we can't send responses back easily
                // For subsession rekeying, the client should use TCP control plane
                debug!(
                    "Ignoring SendPacket action on UDP (receiver_idx={receiver_idx}) - use TCP for rekeying",
                );
                inc!("lp_data_ignored_send_actions");
                Ok(())
            }
            other => {
                warn!(
                    "Unexpected action on UDP data plane from {}: {:?}",
                    src_addr, other
                );
                inc!("lp_data_unexpected_actions");
                Err(GatewayError::LpProtocolError(format!(
                    "Unexpected state machine action on UDP: {:?}",
                    other
                )))
            }
        }
    }

    /// Parse Sphinx packet bytes and forward to mixnet
    ///
    /// The decrypted LP payload contains a serialized MixPacket that includes:
    /// - Packet type (1 byte)
    /// - Key rotation (1 byte)
    /// - Next hop address (first mix node)
    /// - Sphinx packet data
    async fn forward_sphinx_packet(&self, sphinx_bytes: &[u8]) -> Result<(), GatewayError> {
        // Parse as MixPacket v2 format (packet_type || key_rotation || next_hop || packet)
        let mix_packet = MixPacket::try_from_v2_bytes(sphinx_bytes).map_err(|e| {
            inc!("lp_data_sphinx_parse_errors");
            GatewayError::LpProtocolError(format!("Failed to parse MixPacket: {e}"))
        })?;

        trace!(
            "Forwarding Sphinx packet to mixnet (next_hop={}, type={:?})",
            mix_packet.next_hop(),
            mix_packet.packet_type()
        );

        // Forward to mixnet via the shared channel
        if let Err(e) = self.state.outbound_mix_sender.forward_packet(mix_packet) {
            error!("Failed to forward Sphinx packet to mixnet: {}", e);
            inc!("lp_data_forward_errors");
            return Err(GatewayError::InternalError(format!(
                "Mix packet forwarding failed: {e}",
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_max_packet_size_reasonable() {
        // Sphinx packets are typically around 2KB
        // LP overhead is small (~50 bytes header + AEAD tag)
        // 4KB should be plenty with room to spare
        assert!(MAX_UDP_PACKET_SIZE >= 2048 + 100);
    }
}
