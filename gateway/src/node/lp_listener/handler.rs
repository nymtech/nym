// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::messages::LpRegistrationRequest;
use super::registration::process_registration;
use super::LpHandlerState;
use crate::error::GatewayError;
use nym_lp::{
    codec::OuterAeadKey, keypair::PublicKey, message::ForwardPacketData, packet::LpHeader,
    LpMessage, LpPacket, OuterHeader,
};
use nym_lp_transport::traits::LpTransport;
use nym_metrics::{add_histogram_obs, inc};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::*;

// Histogram buckets for LP operation duration (legacy - used by unused forwarding methods)
const LP_DURATION_BUCKETS: &[f64] = &[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];

// Timeout for forward I/O operations (send + receive on exit stream)
// Must be long enough to cover exit gateway processing time
const FORWARD_IO_TIMEOUT_SECS: u64 = 30;

// Histogram buckets for LP connection lifecycle duration
// LP connections can be very short (registration only: ~1s) or very long (dVPN sessions: hours/days)
// Covers full range from seconds to 24 hours
const LP_CONNECTION_DURATION_BUCKETS: &[f64] = &[
    1.0,     // 1 second
    5.0,     // 5 seconds
    10.0,    // 10 seconds
    30.0,    // 30 seconds
    60.0,    // 1 minute
    300.0,   // 5 minutes
    600.0,   // 10 minutes
    1800.0,  // 30 minutes
    3600.0,  // 1 hour
    7200.0,  // 2 hours
    14400.0, // 4 hours
    28800.0, // 8 hours
    43200.0, // 12 hours
    86400.0, // 24 hours
];

/// Connection lifecycle statistics tracking
struct ConnectionStats {
    /// When the connection started
    start_time: std::time::Instant,
    /// Total bytes received (including protocol framing)
    bytes_received: u64,
    /// Total bytes sent (including protocol framing)
    bytes_sent: u64,
}

impl ConnectionStats {
    fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            bytes_received: 0,
            bytes_sent: 0,
        }
    }

    fn record_bytes_received(&mut self, bytes: usize) {
        self.bytes_received += bytes as u64;
    }

    fn record_bytes_sent(&mut self, bytes: usize) {
        self.bytes_sent += bytes as u64;
    }
}

pub struct LpConnectionHandler<S = TcpStream> {
    stream: S,
    remote_addr: SocketAddr,
    state: LpHandlerState,
    stats: ConnectionStats,
    /// Bound receiver_idx for this connection (set after first packet).
    /// All subsequent packets on this connection must use this receiver_idx.
    /// Set from ClientHello's proposed receiver_index, or from header for non-bootstrap packets.
    bound_receiver_idx: Option<u32>,

    /// Persistent connection to exit gateway for forwarding.
    /// Opened on first forward, reused for subsequent forwards, closed when client disconnects.
    /// Tuple contains (stream, target_address) to verify subsequent forwards go to same exit.
    exit_stream: Option<(S, SocketAddr)>,
}

impl<S> LpConnectionHandler<S>
where
    S: LpTransport + Unpin,
{
    pub fn new(stream: S, remote_addr: SocketAddr, state: LpHandlerState) -> Self {
        Self {
            stream,
            remote_addr,
            state,
            stats: ConnectionStats::new(),
            bound_receiver_idx: None,
            exit_stream: None,
        }
    }

    /// AIDEV-NOTE: Stream-oriented packet loop
    /// This handler processes multiple packets on a single TCP connection.
    /// Connection lifecycle: handshake + registration, then client closes.
    /// First packet binds the connection to a receiver_idx (session-affine).
    /// Binding is set by handle_client_hello() from payload's receiver_index,
    /// or by validate_or_set_binding() for non-bootstrap first packets.
    pub async fn handle(mut self) -> Result<(), GatewayError> {
        debug!("Handling LP connection from {}", self.remote_addr);

        // Track total LP connections handled
        inc!("lp_connections_total");

        // ============================================================
        // STREAM-ORIENTED PROCESSING: Loop until connection closes
        // State persists in LpHandlerState maps across packets
        // ============================================================

        loop {
            // Step 1: Receive raw packet bytes and parse header only (for routing)
            let (raw_bytes, header) = match self.receive_raw_packet().await {
                Ok(result) => result,
                Err(e) if Self::is_connection_closed(&e) => {
                    // Graceful EOF - client closed connection
                    trace!("Connection closed by {} (EOF)", self.remote_addr);
                    break;
                }
                Err(e) => {
                    inc!("lp_errors_receive_packet");
                    self.emit_lifecycle_metrics(false);
                    return Err(e);
                }
            };

            let receiver_idx = header.receiver_idx;

            // Step 2: Validate or set binding (session-affine connection)
            // Note: ClientHello (receiver_idx=0) defers binding to handle_client_hello()
            if let Err(e) = self.validate_or_set_binding(receiver_idx) {
                self.emit_lifecycle_metrics(false);
                return Err(e);
            }

            // Step 3: Process the packet
            if let Err(e) = self.process_packet(raw_bytes, receiver_idx).await {
                self.emit_lifecycle_metrics(false);
                return Err(e);
            }
        }

        self.emit_lifecycle_metrics(true);
        Ok(())
    }

    /// Check if an error indicates the connection was closed (EOF).
    /// AIDEV-NOTE: Uses string matching on error messages. Tokio's read_exact
    /// returns UnexpectedEof which gets formatted into the error message.
    fn is_connection_closed(e: &GatewayError) -> bool {
        match e {
            GatewayError::LpConnectionError(msg) => {
                msg.contains("unexpected end of file")
                    || msg.contains("connection reset")
                    || msg.contains("broken pipe")
            }
            _ => false,
        }
    }

    /// Validate that the receiver_idx matches the bound session, or set binding if first packet.
    ///
    /// Binding rules:
    /// - ClientHello (receiver_idx=0): binding deferred to handle_client_hello() which
    ///   extracts receiver_index from payload
    /// - First non-bootstrap packet: sets binding from header's receiver_idx
    /// - Subsequent packets: must match bound receiver_idx
    fn validate_or_set_binding(&mut self, receiver_idx: u32) -> Result<(), GatewayError> {
        match self.bound_receiver_idx {
            None => {
                // First packet - don't bind if bootstrap (handle_client_hello sets binding)
                if receiver_idx != nym_lp::BOOTSTRAP_RECEIVER_IDX {
                    self.bound_receiver_idx = Some(receiver_idx);
                    trace!(
                        "Bound connection from {} to receiver_idx={}",
                        self.remote_addr,
                        receiver_idx
                    );
                }
                Ok(())
            }
            Some(bound) => {
                if receiver_idx == bound {
                    Ok(())
                } else {
                    warn!(
                        "Receiver_idx mismatch from {}: expected {}, got {}",
                        self.remote_addr, bound, receiver_idx
                    );
                    inc!("lp_errors_receiver_idx_mismatch");
                    Err(GatewayError::LpProtocolError(format!(
                        "receiver_idx mismatch: connection bound to {}, packet has {}",
                        bound, receiver_idx
                    )))
                }
            }
        }
    }

    /// Process a single packet: lookup session, parse, route to handler.
    /// Individual handlers do NOT emit lifecycle metrics - the main loop handles that.
    async fn process_packet(
        &mut self,
        raw_bytes: Vec<u8>,
        receiver_idx: u32,
    ) -> Result<(), GatewayError> {
        // Get outer_aead_key based on receiver_idx
        // Header is always cleartext for routing. Payload is encrypted after PSK.
        let outer_key: Option<OuterAeadKey> = if receiver_idx == nym_lp::BOOTSTRAP_RECEIVER_IDX {
            // ClientHello - no encryption (PSK not yet derived)
            None
        } else if let Some(state_entry) = self.state.handshake_states.get(&receiver_idx) {
            // Handshake in progress - check if PSK has been injected yet
            state_entry
                .value()
                .state
                .session()
                .ok()
                .and_then(|session| session.outer_aead_key())
        } else if let Some(session_entry) = self.state.session_states.get(&receiver_idx) {
            // Established session - should always have PSK
            session_entry
                .value()
                .state
                .session()
                .ok()
                .and_then(|s| s.outer_aead_key())
        } else {
            // Unknown session - will error during routing, parse cleartext
            None
        };

        // Parse full packet with outer AEAD key
        let packet =
            nym_lp::codec::parse_lp_packet(&raw_bytes, outer_key.as_ref()).map_err(|e| {
                inc!("lp_errors_parse_packet");
                GatewayError::LpProtocolError(format!("Failed to parse LP packet: {}", e))
            })?;

        trace!(
            "Received packet from {} (receiver_idx={}, counter={}, encrypted={})",
            self.remote_addr,
            receiver_idx,
            packet.header().counter,
            outer_key.is_some()
        );

        // Route packet based on receiver_idx
        if receiver_idx == nym_lp::BOOTSTRAP_RECEIVER_IDX {
            // ClientHello - first packet in handshake
            self.handle_client_hello(packet).await
        } else {
            // Check if this is an in-progress handshake or established session
            if self.state.handshake_states.contains_key(&receiver_idx) {
                // Handshake in progress
                self.handle_handshake_packet(receiver_idx, packet).await
            } else if self.state.session_states.contains_key(&receiver_idx) {
                // Established session - transport mode
                self.handle_transport_packet(receiver_idx, packet).await
            } else {
                // Unknown session - possibly stale or client error
                warn!(
                    "Received packet for unknown session {} from {}",
                    receiver_idx, self.remote_addr
                );
                inc!("lp_errors_unknown_session");
                Err(GatewayError::LpProtocolError(format!(
                    "Unknown session ID: {}",
                    receiver_idx
                )))
            }
        }
    }

    /// Handle ClientHello packet (receiver_idx=0, first packet)
    async fn handle_client_hello(&mut self, packet: LpPacket) -> Result<(), GatewayError> {
        use nym_lp::packet::LpHeader;
        use nym_lp::state_machine::{LpInput, LpStateMachine};

        // Extract ClientHello data
        let (receiver_index, client_ed25519_pubkey, salt) = match packet.message() {
            LpMessage::ClientHello(hello_data) => {
                // Validate timestamp
                let timestamp = hello_data.extract_timestamp();
                Self::validate_timestamp(
                    timestamp,
                    self.state.lp_config.debug.timestamp_tolerance,
                )?;

                // Extract client-proposed receiver_index
                let receiver_index = hello_data.receiver_index;

                let client_ed25519_pubkey = nym_crypto::asymmetric::ed25519::PublicKey::from_bytes(
                    &hello_data.client_ed25519_public_key,
                )
                .map_err(|e| {
                    GatewayError::LpProtocolError(
                        format!("Invalid client Ed25519 public key: {e}",),
                    )
                })?;

                (receiver_index, client_ed25519_pubkey, hello_data.salt)
            }
            other => {
                inc!("lp_client_hello_failed");
                return Err(GatewayError::LpProtocolError(format!(
                    "Expected ClientHello, got {other}",
                )));
            }
        };

        debug!(
            "Processing ClientHello from {} (proposed receiver_index={})",
            self.remote_addr, receiver_index
        );

        // Collision check for client-proposed receiver_index
        // Check both handshake_states (in-progress) and session_states (established)
        if self.state.handshake_states.contains_key(&receiver_index)
            || self.state.session_states.contains_key(&receiver_index)
        {
            warn!(
                "Receiver index collision: {} from {}",
                receiver_index, self.remote_addr
            );
            inc!("lp_receiver_index_collision");

            // Send Collision response to tell client to retry with new receiver_index
            // No outer key - this is before PSK derivation
            // Note: Do NOT set binding on collision - client may retry with new receiver_index
            let collision_packet =
                LpPacket::new(LpHeader::new(receiver_index, 0), LpMessage::Collision);
            self.send_lp_packet(collision_packet, None).await?;

            return Ok(());
        }

        // Collision check passed - bind this connection to the receiver_index
        // All subsequent packets on this connection must use this receiver_index
        self.bound_receiver_idx = Some(receiver_index);
        trace!(
            "Bound connection from {} to receiver_idx={} (via ClientHello)",
            self.remote_addr,
            receiver_index
        );

        // Create state machine for this handshake using client-proposed receiver_index
        let mut state_machine = LpStateMachine::new(
            receiver_index,
            false, // responder
            (
                self.state.local_identity.private_key(),
                self.state.local_identity.public_key(),
            ),
            &client_ed25519_pubkey,
            &salt,
        )
        .map_err(|e| {
            inc!("lp_client_hello_failed");
            GatewayError::LpHandshakeError(format!("Failed to create state machine: {}", e))
        })?;

        debug!(
            "Created handshake state for {} (receiver_index={})",
            self.remote_addr, receiver_index
        );

        // Transition state machine to KKTExchange (responder waits for client's KKT request)
        // For responder, StartHandshake returns None (just transitions state)
        // For initiator, StartHandshake returns SendPacket (KKT request)
        if let Some(Err(e)) = state_machine.process_input(LpInput::StartHandshake) {
            inc!("lp_client_hello_failed");
            return Err(GatewayError::LpHandshakeError(format!(
                "StartHandshake failed: {}",
                e
            )));
            // Responder (gateway) gets Ok but no packet to send - we just wait for client's next packet
        }

        // Store state machine for subsequent handshake packets (KKT request with receiver_index=X)
        self.state
            .handshake_states
            .insert(receiver_index, super::TimestampedState::new(state_machine));

        debug!(
            "Stored handshake state for {} (receiver_index={}) - waiting for KKT request",
            self.remote_addr, receiver_index
        );

        // Send Ack to confirm ClientHello received
        // No outer key - this is before PSK derivation
        let ack_packet = LpPacket::new(LpHeader::new(receiver_index, 0), LpMessage::Ack);
        self.send_lp_packet(ack_packet, None).await?;

        Ok(())
    }

    /// Handle handshake packet (receiver_idx!=0, handshake not complete)
    async fn handle_handshake_packet(
        &mut self,
        receiver_idx: u32,
        packet: LpPacket,
    ) -> Result<(), GatewayError> {
        use nym_lp::state_machine::{LpAction, LpInput};

        debug!(
            "Processing handshake packet from {} (receiver_idx={})",
            self.remote_addr, receiver_idx
        );

        // Get mutable reference to state machine
        let mut state_entry = self
            .state
            .handshake_states
            .get_mut(&receiver_idx)
            .ok_or_else(|| {
                GatewayError::LpProtocolError(format!(
                    "Handshake state not found for session {}",
                    receiver_idx
                ))
            })?;

        let state_machine = &mut state_entry.value_mut().state;

        // Process packet through state machine
        let action = state_machine
            .process_input(LpInput::ReceivePacket(packet))
            .ok_or_else(|| {
                GatewayError::LpHandshakeError("State machine returned no action".to_string())
            })?
            .map_err(|e| GatewayError::LpHandshakeError(format!("Handshake error: {}", e)))?;

        // Get outer_aead_key from session (if PSK has been derived)
        // PSK is derived after Noise msg 1 processing, so msg 2+ are encrypted
        let should_send = match action {
            LpAction::SendPacket(response_packet) => {
                // Get key before dropping borrow
                let outer_key = state_machine
                    .session()
                    .ok()
                    .and_then(|s| s.outer_aead_key());
                drop(state_entry); // Release borrow before send
                Some((response_packet, outer_key))
            }
            LpAction::HandshakeComplete => {
                info!(
                    "Handshake completed for {} (receiver_idx={})",
                    self.remote_addr, receiver_idx
                );

                // Get outer key for Ack encryption before releasing borrow
                let outer_key = state_entry
                    .value()
                    .state
                    .session()
                    .ok()
                    .and_then(|s| s.outer_aead_key());

                // Move state machine to session_states (already in Transport state)
                // We keep the state machine (not just session) to enable
                // subsession/rekeying support during transport phase
                drop(state_entry); // Release mutable borrow

                let (_receiver_idx, timestamped_state) = self
                    .state
                    .handshake_states
                    .remove(&receiver_idx)
                    .ok_or_else(|| {
                        GatewayError::LpHandshakeError(
                            "Failed to remove handshake state".to_string(),
                        )
                    })?;

                self.state
                    .session_states
                    .insert(receiver_idx, timestamped_state);

                inc!("lp_handshakes_success");

                // Send Ack to confirm handshake completion to the client
                let ack_packet = LpPacket::new(LpHeader::new(receiver_idx, 0), LpMessage::Ack);
                trace!(
                    "Moved session {} to transport mode, sending Ack",
                    receiver_idx
                );
                Some((ack_packet, outer_key))
            }
            other => {
                debug!("Received action during handshake: {:?}", other);
                drop(state_entry);
                None
            }
        };

        // Send response packet if needed
        if let Some((packet, outer_key)) = should_send {
            self.send_lp_packet(packet, outer_key.as_ref()).await?;
            trace!(
                "Sent handshake response to {} (encrypted={})",
                self.remote_addr,
                outer_key.is_some()
            );
        }

        Ok(())
    }

    /// Handle transport packet (receiver_idx!=0, session established)
    ///
    /// This handles packets on established sessions, which can be either:
    /// 1. EncryptedData containing LpRegistrationRequest or ForwardPacketData
    /// 2. SubsessionKK1 - Client initiates subsession/rekeying
    /// 3. SubsessionReady - Client confirms subsession promotion
    ///
    /// We process all transport packets through the state machine to enable
    /// subsession support. The state machine returns appropriate actions:
    /// - DeliverData: decrypted application data to process
    /// - SendPacket: subsession response (KK2) to send
    /// - SubsessionComplete: subsession promoted, create new session
    async fn handle_transport_packet(
        &mut self,
        receiver_idx: u32,
        packet: LpPacket,
    ) -> Result<(), GatewayError> {
        use nym_lp::state_machine::{LpAction, LpInput};

        debug!(
            "Processing transport packet from {} (receiver_idx={})",
            self.remote_addr, receiver_idx
        );

        // Get state machine and process packet
        let mut state_entry = self
            .state
            .session_states
            .get_mut(&receiver_idx)
            .ok_or_else(|| {
                GatewayError::LpProtocolError(format!("Session not found: {}", receiver_idx))
            })?;

        // Update last activity timestamp
        state_entry.value().touch();

        let state_machine = &mut state_entry.value_mut().state;

        // Process packet through state machine
        let action = state_machine
            .process_input(LpInput::ReceivePacket(packet))
            .ok_or_else(|| {
                GatewayError::LpProtocolError("No action from state machine".to_string())
            })?
            .map_err(|e| GatewayError::LpProtocolError(format!("State machine error: {}", e)))?;

        // Get outer key before releasing borrow
        let outer_key = state_machine
            .session()
            .map_err(|e| {
                GatewayError::LpProtocolError(format!(
                    "Session unavailable after processing: {}",
                    e
                ))
            })?
            .outer_aead_key();
        drop(state_entry);

        match action {
            LpAction::SendPacket(response_packet) => {
                // Subsession KK2 response - gateway is responder
                // This means we received SubsessionKK1 and are responding
                debug!(
                    "Sending subsession KK2 response to {} (receiver_idx={})",
                    self.remote_addr, receiver_idx
                );
                inc!("lp_subsession_kk2_sent");
                self.send_lp_packet(response_packet, outer_key.as_ref())
                    .await?;
                Ok(())
            }
            LpAction::DeliverData(data) => {
                // Decrypted application data - process as registration/forwarding
                self.handle_decrypted_payload(receiver_idx, data.to_vec())
                    .await
            }
            LpAction::SubsessionComplete {
                packet: ready_packet,
                subsession,
                new_receiver_index,
            } => {
                // Subsession complete - promote to new session
                self.handle_subsession_complete(
                    receiver_idx,
                    ready_packet,
                    *subsession,
                    new_receiver_index,
                    outer_key,
                )
                .await
            }
            other => {
                warn!(
                    "Unexpected action in transport from {}: {:?}",
                    self.remote_addr, other
                );
                Err(GatewayError::LpProtocolError(format!(
                    "Unexpected action: {:?}",
                    other
                )))
            }
        }
    }

    /// Handle decrypted transport payload (registration or forwarding request)
    async fn handle_decrypted_payload(
        &mut self,
        receiver_idx: u32,
        decrypted_bytes: Vec<u8>,
    ) -> Result<(), GatewayError> {
        let remote = self.remote_addr;

        // Try to deserialize as LpRegistrationRequest first (most common case after handshake)
        if let Ok(request) = LpRegistrationRequest::try_deserialise(&decrypted_bytes) {
            debug!(
                "LP registration request from {remote} (receiver_idx={receiver_idx}): mode={:?}",
                request.mode
            );
            return self
                .handle_registration_request(receiver_idx, request)
                .await;
        }

        // Try to deserialize as ForwardPacketData (entry gateway forwarding to exit)
        if let Ok(forward_data) = ForwardPacketData::decode(&decrypted_bytes) {
            debug!(
                "LP forward request from {remote} (receiver_idx={receiver_idx}) to {}",
                forward_data.target_lp_address
            );
            return self
                .handle_forwarding_request(receiver_idx, forward_data)
                .await;
        }

        // Neither registration nor forwarding - unknown payload type
        warn!("Unknown transport payload type from {remote} (receiver_idx={receiver_idx})");
        inc!("lp_errors_unknown_payload_type");
        Err(GatewayError::LpProtocolError(
            "Unknown transport payload type (not registration or forwarding)".to_string(),
        ))
    }

    /// Handle subsession completion - promote subsession to new session
    ///
    /// When a subsession handshake completes (SubsessionReady received):
    /// 1. Send SubsessionReady packet if present (for initiator - gateway is responder, so None)
    /// 2. Create new state machine from completed subsession
    /// 3. Store new session under new_receiver_index
    /// 4. Old session stays in ReadOnlyTransport state until TTL cleanup
    async fn handle_subsession_complete(
        &mut self,
        old_receiver_idx: u32,
        ready_packet: Option<LpPacket>,
        subsession: nym_lp::session::SubsessionHandshake,
        new_receiver_index: u32,
        outer_key: Option<nym_lp::codec::OuterAeadKey>,
    ) -> Result<(), GatewayError> {
        use nym_lp::state_machine::LpStateMachine;

        info!(
            "Subsession complete from {}: old_idx={}, new_idx={}",
            self.remote_addr, old_receiver_idx, new_receiver_index
        );

        // Send SubsessionReady packet if present (for initiator - gateway is responder, so typically None)
        if let Some(packet) = ready_packet {
            self.send_lp_packet(packet, outer_key.as_ref()).await?;
        }

        // Create new state machine from completed subsession
        let new_state_machine = LpStateMachine::from_subsession(subsession, new_receiver_index)
            .map_err(|e| {
                GatewayError::LpProtocolError(format!(
                    "Failed to create session from subsession: {}",
                    e
                ))
            })?;

        // Check for receiver_index collision before inserting
        // new_receiver_index is client-generated (rand::random() in state machine).
        // Collisions are statistically unlikely (1 in 4 billion) but could cause DoS if exploited.
        if self.state.session_states.contains_key(&new_receiver_index)
            || self
                .state
                .handshake_states
                .contains_key(&new_receiver_index)
        {
            warn!(
                "Subsession receiver_index collision: {} from {}",
                new_receiver_index, self.remote_addr
            );
            inc!("lp_subsession_receiver_index_collision");
            return Err(GatewayError::LpProtocolError(
                "Subsession receiver index collision - client should retry".to_string(),
            ));
        }

        // Store new session under new_receiver_index
        self.state.session_states.insert(
            new_receiver_index,
            super::TimestampedState::new(new_state_machine),
        );

        // Old session is now in ReadOnlyTransport state (handled by state machine)
        // It will be cleaned up by TTL-based cleanup task

        inc!("lp_subsession_complete");
        Ok(())
    }

    /// Handle registration request on an established session
    async fn handle_registration_request(
        &mut self,
        receiver_idx: u32,
        request: LpRegistrationRequest,
    ) -> Result<(), GatewayError> {
        // Process registration (might modify state)
        let response = process_registration(request, &self.state).await;

        // Acquire session lock for encryption and get outer AEAD key
        let (response_packet, outer_key) = {
            let session_entry = self
                .state
                .session_states
                .get(&receiver_idx)
                .ok_or_else(|| {
                    GatewayError::LpProtocolError(format!("Session not found: {}", receiver_idx))
                })?;
            // Access session via state machine for subsession support
            let session = session_entry
                .value()
                .state
                .session()
                .map_err(|e| GatewayError::LpProtocolError(format!("Session error: {}", e)))?;

            // Serialize and encrypt response
            let response_bytes = response.serialise().map_err(|e| {
                GatewayError::LpProtocolError(format!("Failed to serialize response: {}", e))
            })?;

            let encrypted_message = session.encrypt_data(&response_bytes).map_err(|e| {
                GatewayError::LpProtocolError(format!("Failed to encrypt response: {}", e))
            })?;

            let packet = session.next_packet(encrypted_message).map_err(|e| {
                GatewayError::LpProtocolError(format!("Failed to create response packet: {}", e))
            })?;

            // Get outer AEAD key for packet encryption
            let outer_key = session.outer_aead_key();
            (packet, outer_key)
        };

        // Send response (encrypted with outer AEAD)
        self.send_lp_packet(response_packet, outer_key.as_ref())
            .await?;

        if response.success {
            info!("LP registration successful for {})", self.remote_addr);
        } else {
            warn!(
                "LP registration failed for {}: {:?}",
                self.remote_addr, response.error
            );
        }

        Ok(())
    }

    /// Handle forwarding request on an established session
    ///
    /// Entry gateway receives ForwardPacketData from client, forwards inner packet
    /// to exit gateway, receives response, encrypts it, and sends back to client.
    async fn handle_forwarding_request(
        &mut self,
        receiver_idx: u32,
        forward_data: ForwardPacketData,
    ) -> Result<(), GatewayError> {
        // Forward the packet to the target gateway
        let response_bytes = self.handle_forward_packet(forward_data).await?;

        // Encrypt response for client and get outer AEAD key
        let (response_packet, outer_key) = {
            let session_entry = self
                .state
                .session_states
                .get(&receiver_idx)
                .ok_or_else(|| {
                    GatewayError::LpProtocolError(format!("Session not found: {}", receiver_idx))
                })?;
            // Access session via state machine for subsession support
            let session = session_entry
                .value()
                .state
                .session()
                .map_err(|e| GatewayError::LpProtocolError(format!("Session error: {}", e)))?;

            let encrypted_message = session.encrypt_data(&response_bytes).map_err(|e| {
                GatewayError::LpProtocolError(format!("Failed to encrypt forward response: {}", e))
            })?;

            let packet = session.next_packet(encrypted_message).map_err(|e| {
                GatewayError::LpProtocolError(format!("Failed to create response packet: {}", e))
            })?;

            // Get outer AEAD key for packet encryption
            let outer_key = session.outer_aead_key();
            (packet, outer_key)
        };

        // Send encrypted response to client (encrypted with outer AEAD)
        self.send_lp_packet(response_packet, outer_key.as_ref())
            .await?;

        debug!(
            "LP forwarding completed for {} (receiver_idx={})",
            self.remote_addr, receiver_idx
        );

        Ok(())
    }

    /// Validates that a ClientHello timestamp is within the acceptable time window.
    ///
    /// # Arguments
    /// * `client_timestamp` - Unix timestamp (seconds) from ClientHello salt
    /// * `tolerance` - Maximum acceptable age
    ///
    /// # Returns
    /// * `Ok(())` if timestamp is valid (within tolerance window)
    /// * `Err(GatewayError)` if timestamp is too old or too far in the future
    ///
    /// # Security
    /// This prevents replay attacks by rejecting stale ClientHello messages.
    /// The tolerance window should be:
    /// - Large enough for clock skew + network latency
    /// - Small enough to limit replay attack window
    fn validate_timestamp(client_timestamp: u64, tolerance: Duration) -> Result<(), GatewayError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let age = now.abs_diff(client_timestamp);
        if age > tolerance.as_secs() {
            let direction = if now >= client_timestamp {
                "old"
            } else {
                "future"
            };

            // Track timestamp validation failures
            inc!("lp_timestamp_validation_rejected");
            if now >= client_timestamp {
                inc!("lp_errors_timestamp_too_old");
            } else {
                inc!("lp_errors_timestamp_too_far_future");
            }

            return Err(GatewayError::LpProtocolError(format!(
                "ClientHello timestamp is too {} (age: {}s, tolerance: {}s)",
                direction,
                age,
                tolerance.as_secs()
            )));
        }

        // Track successful timestamp validation
        inc!("lp_timestamp_validation_accepted");
        Ok(())
    }

    /// Receive client's public key and salt via ClientHello message
    ///
    /// Note: This method is currently unused but retained for potential future use
    /// in alternative handshake flows. The current implementation uses `handle_client_hello()`
    /// which processes ClientHello as part of the single-packet model.
    #[allow(dead_code)]
    async fn receive_client_hello(
        &mut self,
    ) -> Result<
        (
            PublicKey,
            nym_crypto::asymmetric::ed25519::PublicKey,
            [u8; 32],
        ),
        GatewayError,
    > {
        // Receive first packet which should be ClientHello (no outer encryption)
        let (raw_bytes, _header) = self.receive_raw_packet().await?;
        let packet = nym_lp::codec::parse_lp_packet(&raw_bytes, None)
            .map_err(|e| GatewayError::LpProtocolError(format!("Failed to parse packet: {}", e)))?;

        // Verify it's a ClientHello message
        match packet.message() {
            LpMessage::ClientHello(hello_data) => {
                // Extract and validate timestamp (nym-110: replay protection)
                let timestamp = hello_data.extract_timestamp();
                Self::validate_timestamp(
                    timestamp,
                    self.state.lp_config.debug.timestamp_tolerance,
                )?;

                tracing::debug!(
                    "ClientHello timestamp validated: {} (age: {}s, tolerance: {}s)",
                    timestamp,
                    {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
                        now.abs_diff(timestamp)
                    },
                    self.state.lp_config.debug.timestamp_tolerance.as_secs()
                );

                // Convert bytes to X25519 PublicKey (for Noise protocol)
                let client_pubkey = PublicKey::from_bytes(&hello_data.client_lp_public_key)
                    .map_err(|e| {
                        GatewayError::LpProtocolError(format!("Invalid client public key: {}", e))
                    })?;

                // Convert bytes to Ed25519 PublicKey (for PSQ authentication)
                let client_ed25519_pubkey = nym_crypto::asymmetric::ed25519::PublicKey::from_bytes(
                    &hello_data.client_ed25519_public_key,
                )
                .map_err(|e| {
                    GatewayError::LpProtocolError(format!(
                        "Invalid client Ed25519 public key: {}",
                        e
                    ))
                })?;

                // Extract salt for PSK derivation
                let salt = hello_data.salt;

                Ok((client_pubkey, client_ed25519_pubkey, salt))
            }
            other => Err(GatewayError::LpProtocolError(format!(
                "Expected ClientHello, got {}",
                other
            ))),
        }
    }

    /// Returns reference to the established forwarding channel to the exit.
    pub fn forwarding_channel(&self) -> &Option<(S, SocketAddr)> {
        &self.exit_stream
    }

    /// This method establishes connection to the target gateway in order to
    /// forward received packets and retrieve any responses
    //
    // In the future it will also perform identity validation to make sure
    // the target node is a valid gateway present in the network
    //
    // Do not manually call this function. It is only exposed for the purposes of integration tests
    #[doc(hidden)]
    pub async fn establish_exit_stream(
        &mut self,
        target_addr: SocketAddr,
    ) -> Result<(), GatewayError> {
        // Acquire semaphore permit to limit concurrent connection opens (FD exhaustion protection)
        // Permit is scoped to this block - only protects the connect() call, not stream reuse
        let _permit = match self.state.forward_semaphore.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                inc!("lp_forward_rejected");
                return Err(GatewayError::LpConnectionError(
                    "Gateway at forward capacity".into(),
                ));
            }
        };

        // Connect to target gateway with timeout
        let stream = match timeout(Duration::from_secs(5), S::connect(target_addr)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                inc!("lp_forward_failed");
                return Err(GatewayError::LpConnectionError(format!(
                    "Failed to connect to target gateway: {e}",
                )));
            }
            Err(_) => {
                inc!("lp_forward_failed");
                return Err(GatewayError::LpConnectionError(
                    "Target gateway connection timeout".to_string(),
                ));
            }
        };

        debug!("Opened persistent exit connection to {target_addr} for forwarding");
        self.exit_stream = Some((stream, target_addr));

        Ok(())
    }

    /// Forward an LP packet to another gateway
    ///
    /// This method connects to the target gateway, forwards the inner packet bytes,
    /// receives the response, and returns it. Used for telescoping (hiding client IP).
    ///
    /// # Arguments
    /// * `forward_data` - ForwardPacketData containing target gateway info and inner packet
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Raw response bytes from target gateway
    /// * `Err(GatewayError)` - If forwarding fails
    ///
    /// AIDEV-NOTE: Persistent exit stream forwarding
    /// Uses self.exit_stream to maintain a persistent connection to the exit gateway.
    /// First forward opens the connection, subsequent forwards reuse it.
    /// Connection errors clear exit_stream, causing reconnection on next forward.
    ///
    /// Semaphore rationale: The forward_semaphore limits concurrent connection OPENS
    /// (FD exhaustion protection), not concurrent operations. Since:
    /// 1. Each LpConnectionHandler owns its exit_stream exclusively
    /// 2. The handler loop processes packets sequentially (no concurrent access)
    /// 3. Only connection opens consume new FDs
    ///
    /// The semaphore is only acquired when opening a new connection, not for reuse.
    async fn handle_forward_packet(
        &mut self,
        forward_data: ForwardPacketData,
    ) -> Result<Vec<u8>, GatewayError> {
        use std::time::Duration;
        use tokio::time::timeout;

        inc!("lp_forward_total");
        let start = std::time::Instant::now();

        // Parse target gateway address
        let target_addr: SocketAddr = forward_data.target_lp_address.parse().map_err(|e| {
            inc!("lp_forward_failed");
            GatewayError::LpProtocolError(format!("Invalid target address: {}", e))
        })?;

        // Check if we need to open a new connection
        let need_new_connection = match &self.exit_stream {
            Some((_, existing_addr)) if *existing_addr == target_addr => false,
            Some((_, existing_addr)) => {
                // Target mismatch - this shouldn't happen in normal operation
                // (client should only forward to one exit gateway)
                // Return error to prevent silent behavior changes that could mask bugs
                inc!("lp_forward_failed");
                return Err(GatewayError::LpProtocolError(format!(
                    "Forward target mismatch: session bound to {existing_addr}, got request for {target_addr}"
                )));
            }
            None => true,
        };

        if need_new_connection {
            self.establish_exit_stream(target_addr).await?;
        }

        // Get mutable reference to the exit stream
        #[allow(clippy::unwrap_used)]
        let (target_stream, _) = self.exit_stream.as_mut().unwrap();

        debug!(
            "Forwarding packet to {} ({} bytes)",
            target_addr,
            forward_data.inner_packet_bytes.len()
        );

        // Wrap all I/O in timeout to prevent hanging on unresponsive exit gateway
        let io_timeout = Duration::from_secs(FORWARD_IO_TIMEOUT_SECS);
        let inner_bytes = &forward_data.inner_packet_bytes;

        let io_result: Result<Vec<u8>, GatewayError> = timeout(io_timeout, async {
            // Forward inner packet bytes (4-byte length prefix + packet data)
            let len = inner_bytes.len() as u32;
            target_stream
                .write_all(&len.to_be_bytes())
                .await
                .map_err(|e| {
                    GatewayError::LpConnectionError(format!(
                        "Failed to send length to target: {}",
                        e
                    ))
                })?;

            target_stream.write_all(inner_bytes).await.map_err(|e| {
                GatewayError::LpConnectionError(format!("Failed to send packet to target: {}", e))
            })?;

            target_stream.flush().await.map_err(|e| {
                GatewayError::LpConnectionError(format!("Failed to flush target stream: {}", e))
            })?;

            // Read response from target gateway (4-byte length prefix + packet data)
            let mut len_buf = [0u8; 4];
            target_stream.read_exact(&mut len_buf).await.map_err(|e| {
                GatewayError::LpConnectionError(format!(
                    "Failed to read response length from target: {}",
                    e
                ))
            })?;

            let response_len = u32::from_be_bytes(len_buf) as usize;

            // Sanity check
            const MAX_PACKET_SIZE: usize = 65536;
            if response_len > MAX_PACKET_SIZE {
                return Err(GatewayError::LpProtocolError(format!(
                    "Response size {response_len} exceeds maximum {MAX_PACKET_SIZE}",
                )));
            }

            let mut response_buf = vec![0u8; response_len];
            target_stream
                .read_exact(&mut response_buf)
                .await
                .map_err(|e| {
                    GatewayError::LpConnectionError(format!(
                        "Failed to read response from target: {e}",
                    ))
                })?;

            Ok(response_buf)
        })
        .await
        .unwrap_or_else(|_| {
            Err(GatewayError::LpConnectionError(
                "Forward I/O timeout".to_string(),
            ))
        });

        // Handle result - clear exit_stream on any error
        let response_buf = match io_result {
            Ok(buf) => buf,
            Err(e) => {
                inc!("lp_forward_failed");
                self.exit_stream = None;
                return Err(e);
            }
        };

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        add_histogram_obs!("lp_forward_duration_seconds", duration, LP_DURATION_BUCKETS);

        inc!("lp_forward_success");
        debug!(
            "Forwarding successful to {} ({} bytes response, {:.3}s)",
            target_addr,
            response_buf.len(),
            duration
        );

        Ok(response_buf)
    }

    /// Receive raw packet bytes and parse outer header only (for routing before session lookup).
    ///
    /// Returns the raw packet bytes and parsed outer header (receiver_idx + counter).
    /// The caller should look up the session to get outer_aead_key, then call
    /// `parse_lp_packet()` with the key.
    async fn receive_raw_packet(&mut self) -> Result<(Vec<u8>, OuterHeader), GatewayError> {
        use nym_lp::codec::parse_lp_header_only;

        // Read 4-byte length prefix (u32 big-endian)
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await.map_err(|e| {
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
        self.stream.read_exact(&mut packet_buf).await.map_err(|e| {
            GatewayError::LpConnectionError(format!("Failed to read packet data: {}", e))
        })?;

        // Track bytes received (4 byte header + packet data)
        self.stats.record_bytes_received(4 + packet_len);

        // Parse header only (for routing - header is always cleartext)
        let header = parse_lp_header_only(&packet_buf).map_err(|e| {
            GatewayError::LpProtocolError(format!("Failed to parse LP header: {}", e))
        })?;

        Ok((packet_buf, header))
    }

    /// Send an LP packet over the stream with proper length-prefixed framing.
    ///
    /// # Arguments
    /// * `packet` - The LP packet to send
    /// * `outer_key` - Optional outer AEAD key for encryption (None for cleartext, Some for encrypted)
    async fn send_lp_packet(
        &mut self,
        packet: LpPacket,
        outer_key: Option<&OuterAeadKey>,
    ) -> Result<(), GatewayError> {
        use bytes::BytesMut;
        use nym_lp::codec::serialize_lp_packet;

        // Serialize the packet (encrypted if outer_key provided)
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(&packet, &mut packet_buf, outer_key).map_err(|e| {
            GatewayError::LpProtocolError(format!("Failed to serialize packet: {}", e))
        })?;

        // Send 4-byte length prefix (u32 big-endian)
        let len = packet_buf.len() as u32;
        self.stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| {
                GatewayError::LpConnectionError(format!("Failed to send packet length: {}", e))
            })?;

        // Send the actual packet data
        self.stream.write_all(&packet_buf).await.map_err(|e| {
            GatewayError::LpConnectionError(format!("Failed to send packet data: {}", e))
        })?;

        self.stream.flush().await.map_err(|e| {
            GatewayError::LpConnectionError(format!("Failed to flush stream: {}", e))
        })?;

        // Track bytes sent (4 byte header + packet data)
        self.stats.record_bytes_sent(4 + packet_buf.len());

        Ok(())
    }

    /// Emit connection lifecycle metrics
    fn emit_lifecycle_metrics(&self, graceful: bool) {
        use nym_metrics::inc_by;

        // Track connection duration
        let duration = self.stats.start_time.elapsed().as_secs_f64();
        add_histogram_obs!(
            "lp_connection_duration_seconds",
            duration,
            LP_CONNECTION_DURATION_BUCKETS
        );

        // Track bytes transferred
        inc_by!(
            "lp_connection_bytes_received_total",
            self.stats.bytes_received as i64
        );
        inc_by!(
            "lp_connection_bytes_sent_total",
            self.stats.bytes_sent as i64
        );

        // Track completion type
        if graceful {
            inc!("lp_connections_completed_gracefully");
        } else {
            inc!("lp_connections_completed_with_error");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::lp_listener::{LpConfig, LpDebug};
    use crate::node::ActiveClientsStore;
    use bytes::BytesMut;
    use nym_lp::codec::{parse_lp_packet, serialize_lp_packet};
    use nym_lp::message::{ClientHelloData, EncryptedDataPayload, HandshakeData, LpMessage};
    use nym_lp::packet::{LpHeader, LpPacket};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

    // ==================== Test Helpers ====================

    /// Create a minimal test state for handler tests
    async fn create_minimal_test_state() -> LpHandlerState {
        use nym_crypto::asymmetric::ed25519;
        use rand::rngs::OsRng;

        // Create in-memory storage for testing
        let storage = nym_gateway_storage::GatewayStorage::init(":memory:", 100)
            .await
            .expect("Failed to create test storage");

        // Create mock ecash manager for testing
        let ecash_verifier =
            nym_credential_verification::ecash::MockEcashManager::new(Box::new(storage.clone()));

        let lp_config = LpConfig {
            debug: LpDebug {
                timestamp_tolerance: Duration::from_secs(30),
                ..Default::default()
            },
            ..Default::default()
        };
        let forward_semaphore = Arc::new(tokio::sync::Semaphore::new(
            lp_config.debug.max_concurrent_forwards,
        ));

        // Create mix forwarding channel (unused in tests but required by struct)
        let (mix_sender, _mix_receiver) = nym_mixnet_client::forwarder::mix_forwarding_channels();

        LpHandlerState {
            lp_config,
            ecash_verifier: Arc::new(ecash_verifier)
                as Arc<dyn nym_credential_verification::ecash::traits::EcashManager + Send + Sync>,
            storage,
            local_identity: Arc::new(ed25519::KeyPair::new(&mut OsRng)),
            metrics: nym_node_metrics::NymNodeMetrics::default(),
            active_clients_store: ActiveClientsStore::new(),
            wg_peer_controller: None,
            wireguard_data: None,
            outbound_mix_sender: mix_sender,
            handshake_states: Arc::new(dashmap::DashMap::new()),
            session_states: Arc::new(dashmap::DashMap::new()),
            forward_semaphore,
        }
    }

    /// Helper to write an LP packet to a stream with proper framing
    async fn write_lp_packet_to_stream<W: AsyncWriteExt + Unpin>(
        stream: &mut W,
        packet: &LpPacket,
    ) -> Result<(), std::io::Error> {
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(packet, &mut packet_buf, None)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        // Write length prefix
        let len = packet_buf.len() as u32;
        stream.write_all(&len.to_be_bytes()).await?;

        // Write packet data
        stream.write_all(&packet_buf).await?;
        stream.flush().await?;

        Ok(())
    }

    /// Helper to read an LP packet from a stream with proper framing
    async fn read_lp_packet_from_stream<R: AsyncRead + Unpin>(
        stream: &mut R,
    ) -> Result<LpPacket, std::io::Error> {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let packet_len = u32::from_be_bytes(len_buf) as usize;

        // Read packet data
        let mut packet_buf = vec![0u8; packet_len];
        stream.read_exact(&mut packet_buf).await?;

        // Parse packet
        parse_lp_packet(&packet_buf, None).map_err(|e| std::io::Error::other(e.to_string()))
    }

    // ==================== Existing Tests ====================

    #[test]
    fn test_validate_timestamp_current() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Current timestamp should always pass
        assert!(
            LpConnectionHandler::<TcpStream>::validate_timestamp(now, Duration::from_secs(30))
                .is_ok()
        );
    }

    #[test]
    fn test_validate_timestamp_within_tolerance() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 10 seconds old, tolerance 30s -> should pass
        let old_timestamp = now - 10;
        assert!(LpConnectionHandler::<TcpStream>::validate_timestamp(
            old_timestamp,
            Duration::from_secs(30)
        )
        .is_ok());

        // 10 seconds in future, tolerance 30s -> should pass
        let future_timestamp = now + 10;
        assert!(LpConnectionHandler::<TcpStream>::validate_timestamp(
            future_timestamp,
            Duration::from_secs(30)
        )
        .is_ok());
    }

    #[test]
    fn test_validate_timestamp_too_old() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 60 seconds old, tolerance 30s -> should fail
        let old_timestamp = now - 60;
        let result = LpConnectionHandler::<TcpStream>::validate_timestamp(
            old_timestamp,
            Duration::from_secs(30),
        );
        assert!(result.is_err());
        assert!(format!("{:?}", result).contains("too old"));
    }

    #[test]
    fn test_validate_timestamp_too_far_future() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 60 seconds in future, tolerance 30s -> should fail
        let future_timestamp = now + 60;
        let result = LpConnectionHandler::<TcpStream>::validate_timestamp(
            future_timestamp,
            Duration::from_secs(30),
        );
        assert!(result.is_err());
        assert!(format!("{:?}", result).contains("too future"));
    }

    #[test]
    fn test_validate_timestamp_boundary() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Exactly at tolerance boundary -> should pass
        let boundary_timestamp = now - 30;
        assert!(LpConnectionHandler::<TcpStream>::validate_timestamp(
            boundary_timestamp,
            Duration::from_secs(30)
        )
        .is_ok());

        // Just beyond boundary -> should fail
        let beyond_timestamp = now - 31;
        assert!(LpConnectionHandler::<TcpStream>::validate_timestamp(
            beyond_timestamp,
            Duration::from_secs(30)
        )
        .is_err());
    }

    // ==================== Packet I/O Tests ====================

    #[tokio::test]
    async fn test_receive_raw_packet_valid() {
        use tokio::net::{TcpListener, TcpStream};

        // Bind to localhost
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn server task
        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            // Two-phase: receive raw bytes + header, then parse full packet
            let (raw_bytes, header) = handler.receive_raw_packet().await?;
            let packet = parse_lp_packet(&raw_bytes, None).map_err(|e| {
                GatewayError::LpProtocolError(format!("Failed to parse packet: {}", e))
            })?;
            Ok::<_, GatewayError>((header, packet))
        });

        // Connect as client
        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Send a valid packet from client side
        let packet = LpPacket::new(
            LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 42,
                counter: 0,
            },
            LpMessage::Busy,
        );
        write_lp_packet_to_stream(&mut client_stream, &packet)
            .await
            .unwrap();

        // Handler should receive and parse it correctly
        // Note: header is OuterHeader (receiver_idx + counter only), not LpHeader
        let (header, received) = server_task.await.unwrap().unwrap();
        assert_eq!(header.receiver_idx, 42);
        assert_eq!(header.counter, 0);
        assert_eq!(received.header().protocol_version, 1);
        assert_eq!(received.header().receiver_idx, 42);
        assert_eq!(received.header().counter, 0);
    }

    #[tokio::test]
    async fn test_receive_raw_packet_exceeds_max_size() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            handler.receive_raw_packet().await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Send a packet size that exceeds MAX_PACKET_SIZE (64KB)
        let oversized_len: u32 = 70000; // > 65536
        client_stream
            .write_all(&oversized_len.to_be_bytes())
            .await
            .unwrap();
        client_stream.flush().await.unwrap();

        // Handler should reject it
        let result = server_task.await.unwrap();
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("exceeds maximum"));
    }

    #[tokio::test]
    async fn test_send_lp_packet_valid() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    reserved: 0,
                    receiver_idx: 99,
                    counter: 5,
                },
                LpMessage::Busy,
            );
            handler.send_lp_packet(packet, None).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Wait for server to send
        server_task.await.unwrap().unwrap();

        // Client should receive it correctly
        let received = read_lp_packet_from_stream(&mut client_stream)
            .await
            .unwrap();
        assert_eq!(received.header().receiver_idx, 99);
        assert_eq!(received.header().counter, 5);
    }

    #[tokio::test]
    async fn test_send_receive_handshake_message() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handshake_data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let expected_data = handshake_data.clone();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    reserved: 0,
                    receiver_idx: 100,
                    counter: 10,
                },
                LpMessage::Handshake(HandshakeData(handshake_data)),
            );
            handler.send_lp_packet(packet, None).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();
        server_task.await.unwrap().unwrap();

        let received = read_lp_packet_from_stream(&mut client_stream)
            .await
            .unwrap();
        assert_eq!(received.header().receiver_idx, 100);
        assert_eq!(received.header().counter, 10);
        match received.message() {
            LpMessage::Handshake(data) => assert_eq!(data, &HandshakeData(expected_data)),
            _ => panic!("Expected Handshake message"),
        }
    }

    #[tokio::test]
    async fn test_send_receive_encrypted_data_message() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let encrypted_payload = vec![42u8; 256];
        let expected_payload = encrypted_payload.clone();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    reserved: 0,
                    receiver_idx: 200,
                    counter: 20,
                },
                LpMessage::EncryptedData(EncryptedDataPayload(encrypted_payload)),
            );
            handler.send_lp_packet(packet, None).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();
        server_task.await.unwrap().unwrap();

        let received = read_lp_packet_from_stream(&mut client_stream)
            .await
            .unwrap();
        assert_eq!(received.header().receiver_idx, 200);
        assert_eq!(received.header().counter, 20);
        match received.message() {
            LpMessage::EncryptedData(data) => {
                assert_eq!(data, &EncryptedDataPayload(expected_payload))
            }
            _ => panic!("Expected EncryptedData message"),
        }
    }

    #[tokio::test]
    async fn test_send_receive_client_hello_message() {
        use nym_lp::message::ClientHelloData;
        use tokio::net::{TcpListener, TcpStream};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_key = [7u8; 32];
        let client_ed25519_key = [8u8; 32];
        let hello_data =
            ClientHelloData::new_with_fresh_salt(client_key, client_ed25519_key, timestamp);
        let expected_salt = hello_data.salt; // Clone salt before moving hello_data

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    reserved: 0,
                    receiver_idx: 300,
                    counter: 30,
                },
                LpMessage::ClientHello(hello_data),
            );
            handler.send_lp_packet(packet, None).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();
        server_task.await.unwrap().unwrap();

        let received = read_lp_packet_from_stream(&mut client_stream)
            .await
            .unwrap();
        assert_eq!(received.header().receiver_idx, 300);
        assert_eq!(received.header().counter, 30);
        match received.message() {
            LpMessage::ClientHello(data) => {
                assert_eq!(data.client_lp_public_key, client_key);
                assert_eq!(data.salt, expected_salt);
            }
            _ => panic!("Expected ClientHello message"),
        }
    }

    // ==================== receive_client_hello Tests ====================

    #[tokio::test]
    async fn test_receive_client_hello_valid() {
        use tokio::net::{TcpListener, TcpStream};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            handler.receive_client_hello().await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Create and send valid ClientHello
        // Create separate Ed25519 keypair and derive X25519 from it (like production code)
        use nym_crypto::asymmetric::ed25519;
        use rand::rngs::OsRng;

        let client_ed25519_keypair = ed25519::KeyPair::new(&mut OsRng);
        let client_x25519_public = client_ed25519_keypair.public_key().to_x25519().unwrap();

        let hello_data = ClientHelloData::new_with_fresh_salt(
            client_x25519_public.to_bytes(),
            client_ed25519_keypair.public_key().to_bytes(),
            timestamp,
        );
        let packet = LpPacket::new(
            LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 0,
                counter: 0,
            },
            LpMessage::ClientHello(hello_data.clone()),
        );
        write_lp_packet_to_stream(&mut client_stream, &packet)
            .await
            .unwrap();

        // Handler should receive and parse it
        let result = server_task.await.unwrap();
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);

        let (x25519_pubkey, ed25519_pubkey, salt) = result.unwrap();
        assert_eq!(x25519_pubkey.as_bytes(), &client_x25519_public.to_bytes());
        assert_eq!(
            ed25519_pubkey.to_bytes(),
            client_ed25519_keypair.public_key().to_bytes()
        );
        assert_eq!(salt, hello_data.salt);
    }

    #[tokio::test]
    async fn test_receive_client_hello_timestamp_too_old() {
        use std::time::{SystemTime, UNIX_EPOCH};
        use tokio::net::{TcpListener, TcpStream};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            handler.receive_client_hello().await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Create ClientHello with old timestamp
        // Use proper separate Ed25519 and X25519 keys (like production code)
        use nym_crypto::asymmetric::ed25519;
        use rand::rngs::OsRng;

        let client_ed25519_keypair = ed25519::KeyPair::new(&mut OsRng);
        let client_x25519_public = client_ed25519_keypair.public_key().to_x25519().unwrap();

        let mut hello_data = ClientHelloData::new_with_fresh_salt(
            client_x25519_public.to_bytes(),
            client_ed25519_keypair.public_key().to_bytes(),
            timestamp,
        );

        // Manually set timestamp to be very old (100 seconds ago)
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 100;
        hello_data.salt[..8].copy_from_slice(&old_timestamp.to_le_bytes());

        let packet = LpPacket::new(
            LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 0,
                counter: 0,
            },
            LpMessage::ClientHello(hello_data),
        );
        write_lp_packet_to_stream(&mut client_stream, &packet)
            .await
            .unwrap();

        // Should fail with timestamp error
        let result = server_task.await.unwrap();
        assert!(result.is_err());
        // Note: Can't use unwrap_err() directly because PublicKey doesn't implement Debug
        // Just check that it failed
        match result {
            Err(e) => {
                let err_msg = format!("{}", e);
                assert!(
                    err_msg.contains("too old"),
                    "Expected 'too old' in error, got: {}",
                    err_msg
                );
            }
            Ok(_) => panic!("Expected error but got success"),
        }
    }
}
