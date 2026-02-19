// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::{LpHandlerState, ReceiverIndex, TimestampedState};
use crate::error::GatewayError;
use bytes::BytesMut;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::codec::serialize_lp_packet;
use nym_lp::message::ApplicationData;
use nym_lp::state_machine::{LpAction, LpData, LpDataKind, LpInput};
use nym_lp::{
    message::ForwardPacketData, LpMessage, LpPacket, LpSession, LpStateMachine, OuterHeader,
};
use nym_lp_transport::traits::LpTransport;
use nym_metrics::{add_histogram_obs, inc};
use nym_registration_common::{LpRegistrationRequest, RegistrationStatus};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
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
    S: LpTransport + AsyncRead + AsyncWrite + Unpin,
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

        // 1. complete KKT/PSQ handshake before doing anything else.
        // bail if it takes too long
        let timeout = self.state.lp_config.debug.handshake_ttl;
        let local_peer = self.state.local_lp_peer.clone();
        let stream = &mut self.stream;

        // TODO:
        let ciphersuite = LpSession::default_ciphersuite();
        let session = match tokio::time::timeout(timeout, async move {
            LpSession::psq_handshake_responder(stream, ciphersuite, local_peer)
                .complete_as_responder()
                .await
        })
        .await
        {
            Err(_timeout) => {
                debug!(
                    "timed out attempting to complete KTT/PSQ handshake with {}",
                    self.remote_addr
                );
                self.emit_lifecycle_metrics(false);
                return Ok(());
            }
            Ok(Err(handshake_failure)) => {
                debug!(
                    "failed to complete KKT/PSQ handshake with {}: {handshake_failure}",
                    self.remote_addr
                );
                self.emit_lifecycle_metrics(false);
                return Ok(());
            }
            Ok(Ok(session)) => session,
        };
        let receiver_idx = session.id();

        // 2. insert the state machine into the shared state
        let state_machine = LpStateMachine::new(session);
        self.state
            .session_states
            .insert(receiver_idx, TimestampedState::new(state_machine));
        self.bound_receiver_idx = Some(receiver_idx);

        // 3. handle any new incoming packet
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

            // Step 2: Validate the binding
            if let Err(e) = self.validate_binding(receiver_idx) {
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

    fn bound_receiver_index(&self) -> Result<u32, GatewayError> {
        self.bound_receiver_idx.ok_or_else(|| {
            GatewayError::LpProtocolError(
                "missing bound receiver index after KKT/PSQ handshake".into(),
            )
        })
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

    /// Validate that the receiver_idx matches the bound session.
    ///
    /// Binding rules:
    /// - ClientHello (receiver_idx=0): binding deferred to handle_client_hello() which
    ///   extracts receiver_index from payload
    /// - First non-bootstrap packet: sets binding from header's receiver_idx
    /// - Subsequent packets: must match bound receiver_idx
    fn validate_binding(&self, receiver_idx: u32) -> Result<(), GatewayError> {
        let bound_receiver_idx = self.bound_receiver_index()?;

        if bound_receiver_idx != receiver_idx {
            warn!(
                "Receiver_idx mismatch from {}: expected {bound_receiver_idx}, got {receiver_idx}",
                self.remote_addr
            );
            inc!("lp_errors_receiver_idx_mismatch");
            return Err(GatewayError::LpProtocolError(format!(
                "receiver_idx mismatch: connection bound to {bound_receiver_idx}, packet has {receiver_idx}",
            )));
        }

        Ok(())
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
        let Some(state_machine) = self.state.session_states.get(&receiver_idx) else {
            // session might have gotten removed due to inactivity
            return Err(GatewayError::LpConnectionError(format!(
                "missing session state for {receiver_idx} - has it been removed due to inactivity?"
            )));
        };

        let outer_key = state_machine
            .state
            .session()
            .map_err(|err| GatewayError::LpProtocolError(err.to_string()))?
            .outer_aead_key();

        // Parse full packet with outer AEAD key
        let packet = nym_lp::codec::parse_lp_packet(&raw_bytes, Some(outer_key)).map_err(|e| {
            inc!("lp_errors_parse_packet");
            GatewayError::LpProtocolError(format!("Failed to parse LP packet: {e}"))
        })?;

        drop(state_machine);

        trace!(
            "Received packet from {} (receiver_idx={}, counter={})",
            self.remote_addr,
            receiver_idx,
            packet.header().counter,
        );

        self.handle_transport_packet(receiver_idx, packet).await
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
        let remote = self.remote_addr;
        debug!("Processing transport packet from {remote} (receiver_idx={receiver_idx})",);

        // Get state machine and process packet
        let mut state_entry = self
            .state
            .session_states
            .get_mut(&receiver_idx)
            .ok_or_else(|| {
                GatewayError::LpProtocolError(format!("Session not found: {receiver_idx}"))
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
            .map_err(|e| GatewayError::LpProtocolError(format!("State machine error: {e}")))?;

        let lp_session = state_machine.session().map_err(|e| {
            GatewayError::LpProtocolError(format!("Session unavailable after processing: {e}"))
        })?;

        // Get outer key before releasing borrow
        let outer_key = lp_session.outer_aead_key();

        match action {
            LpAction::SendPacket(response_packet) => {
                // Subsession KK2 response - gateway is responder
                // This means we received SubsessionKK1 and are responding
                debug!("Sending subsession KK2 response to {remote} (receiver_idx={receiver_idx})",);
                inc!("lp_subsession_kk2_sent");
                let mut packet_buf = BytesMut::new();
                serialize_lp_packet(&response_packet, &mut packet_buf, Some(outer_key)).map_err(
                    |e| GatewayError::LpProtocolError(format!("Failed to serialize packet: {}", e)),
                )?;

                drop(state_entry);
                self.send_serialised_packet(&packet_buf).await?;
                Ok(())
            }
            LpAction::DeliverData(data) => {
                // Decrypted application data - process as registration/forwarding
                drop(state_entry);
                self.handle_decrypted_payload(receiver_idx, data).await
            }
            LpAction::SubsessionComplete {
                packet: ready_packet,
                subsession,
                new_receiver_index,
            } => {
                // Subsession complete - promote to new session

                // Send SubsessionReady packet if present (for initiator - gateway is responder, so typically None)
                if let Some(ready_packet) = ready_packet {
                    let mut packet_buf = BytesMut::new();
                    serialize_lp_packet(&ready_packet, &mut packet_buf, Some(outer_key)).map_err(
                        |e| {
                            GatewayError::LpProtocolError(format!(
                                "Failed to serialize packet: {e}",
                            ))
                        },
                    )?;
                    drop(state_entry);
                    self.send_serialised_packet(&packet_buf).await?;
                } else {
                    drop(state_entry);
                }
                self.handle_subsession_complete(receiver_idx, *subsession, new_receiver_index)
                    .await
            }
            other => {
                warn!("Unexpected action in transport from {remote}: {other:?}",);
                Err(GatewayError::LpProtocolError(format!(
                    "Unexpected action: {other:?}",
                )))
            }
        }
    }

    /// Handle decrypted transport payload (registration or forwarding request)
    async fn handle_decrypted_payload(
        &mut self,
        receiver_idx: u32,
        decrypted_data: LpData,
    ) -> Result<(), GatewayError> {
        let remote = self.remote_addr;

        let bytes = decrypted_data.content;
        match decrypted_data.kind {
            LpDataKind::Registration => {
                let request = LpRegistrationRequest::try_deserialise(&bytes).map_err(|err| {
                    GatewayError::LpProtocolError(format!("malformed LpRegistrationRequest: {err}"))
                })?;

                debug!(
                    "LP registration request from {remote} (receiver_idx={receiver_idx}): mode={:?}",
                request.mode());

                self.handle_registration_request(receiver_idx, request)
                    .await
            }
            LpDataKind::Forward => {
                let forward_data = ForwardPacketData::decode(&bytes).map_err(|err| {
                    GatewayError::LpProtocolError(format!("malformed ForwardPacketData: {err}"))
                })?;

                self.handle_forwarding_request(receiver_idx, forward_data)
                    .await
            }
            LpDataKind::Opaque => {
                // Neither registration nor forwarding - unknown payload type
                warn!("Unknown transport payload type from {remote} (receiver_idx={receiver_idx}). dropping {} bytes", bytes.len());
                inc!("lp_errors_unknown_payload_type");
                Err(GatewayError::LpProtocolError(
                    "Unknown transport payload type (not registration or forwarding)".to_string(),
                ))
            }
        }
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
        subsession: nym_lp::session::SubsessionHandshake,
        new_receiver_index: u32,
    ) -> Result<(), GatewayError> {
        use nym_lp::state_machine::LpStateMachine;

        info!(
            "Subsession complete from {}: old_idx={}, new_idx={}",
            self.remote_addr, old_receiver_idx, new_receiver_index
        );

        // Create new state machine from completed subsession
        let new_state_machine = LpStateMachine::from_subsession(subsession, new_receiver_index)
            .map_err(|e| {
                GatewayError::LpProtocolError(format!(
                    "Failed to create session from subsession: {e}",
                ))
            })?;

        // Check for receiver_index collision before inserting
        // new_receiver_index is client-generated (rand::random() in state machine).
        // Collisions are statistically unlikely (1 in 4 billion) but could cause DoS if exploited.
        if self.state.session_states.contains_key(&new_receiver_index) {
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

    /// Attempt to wrap and send specified response back to the client
    async fn send_response_packet(
        &mut self,
        receiver_idx: u32,
        serialised_response: Vec<u8>,
        response_kind: LpDataKind,
    ) -> Result<(), GatewayError> {
        let mut session_entry = self
            .state
            .session_states
            .get_mut(&receiver_idx)
            .ok_or_else(|| {
                GatewayError::LpProtocolError(format!("Session not found: {receiver_idx}"))
            })?;

        // Access session via state machine for subsession support
        let session = session_entry
            .value_mut()
            .state
            .session_mut()
            .map_err(|e| GatewayError::LpProtocolError(format!("Session error: {e}")))?;

        let wrapped_lp_data = LpData::new(response_kind, serialised_response);
        let data_bytes = wrapped_lp_data.to_vec();

        let encrypted_message = session.encrypt_application_data(data_bytes).map_err(|e| {
            GatewayError::LpProtocolError(format!("Failed to encrypt response: {e}"))
        })?;

        // make sure to drop the entry before the .await call
        // Serialize the packet (encrypted if outer_key provided)
        let mut packet_buf = BytesMut::new();
        encrypted_message.encode(&mut packet_buf);

        // Send response (encrypted with outer AEAD)
        self.send_serialised_packet(&packet_buf).await?;
        Ok(())
    }

    /// Handle registration request on an established session
    async fn handle_registration_request(
        &mut self,
        receiver_idx: ReceiverIndex,
        request: LpRegistrationRequest,
    ) -> Result<(), GatewayError> {
        // Process registration (might modify state)
        let response = self.state.process_registration(receiver_idx, request).await;
        let response_bytes = response.serialise().map_err(|e| {
            GatewayError::LpProtocolError(format!("Failed to serialize response: {e}"))
        })?;

        self.send_response_packet(receiver_idx, response_bytes, LpDataKind::Registration)
            .await?;

        match response.status {
            RegistrationStatus::Completed => {
                info!("LP registration successful for {}", self.remote_addr);
            }
            RegistrationStatus::Failed => {
                warn!(
                    "LP registration failed for {}: {:?}",
                    self.remote_addr,
                    response.error_message()
                );
            }
            RegistrationStatus::PendingMoreData => {
                info!(
                    "we required more deta from {} to complete registration",
                    self.remote_addr
                );
            }
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
        // Forward the packet to the target gateway and retrieve its response
        let response_bytes = self.handle_forward_packet(forward_data).await?;

        self.send_response_packet(receiver_idx, response_bytes, LpDataKind::Forward)
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
    ) -> Result<(x25519::PublicKey, ed25519::PublicKey, [u8; 32]), GatewayError> {
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

                // Retrieve X25519 PublicKey (for Noise protocol)
                let client_pubkey = hello_data.client_lp_public_key;

                // Retrieve Ed25519 PublicKey (for PSQ authentication)
                let client_ed25519_pubkey = hello_data.client_ed25519_public_key;

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
        let target_addr = forward_data.target_lp_address;

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

    /// Send a serialised LP packet over the stream with proper length-prefixed framing.
    async fn send_serialised_packet(&mut self, packet_data: &[u8]) -> Result<(), GatewayError> {
        self.stream
            .send_length_prefixed_packet(packet_data)
            .await
            .map_err(|err| {
                GatewayError::LpConnectionError(format!("failed to send LP packet: {err}"))
            })?;

        // Track bytes sent (4 byte header + packet data)
        self.stats.record_bytes_sent(4 + packet_data.len());

        Ok(())
    }

    // only used in tests
    #[cfg(test)]
    async fn send_lp_packet(&mut self, packet: LpPacket) -> Result<(), GatewayError> {
        let receiver_idx = self.bound_receiver_index()?;

        let mut session_entry = self
            .state
            .session_states
            .get_mut(&receiver_idx)
            .ok_or_else(|| {
                GatewayError::LpProtocolError(format!("Session not found: {receiver_idx}"))
            })?;

        // Access session via state machine for subsession support
        let session = session_entry
            .value_mut()
            .state
            .session_mut()
            .map_err(|e| GatewayError::LpProtocolError(format!("Session error: {e}")))?;

        let outer_key = session.outer_aead_key();

        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(&packet, &mut packet_buf, Some(outer_key)).map_err(|e| {
            GatewayError::LpProtocolError(format!("Failed to serialize packet: {e}",))
        })?;

        self.stream
            .send_length_prefixed_packet(&packet_buf)
            .await
            .map_err(|err| {
                GatewayError::LpConnectionError(format!("failed to send LP packet: {err}"))
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
    use nym_lp::codec::{parse_lp_packet, serialize_lp_packet, OuterAeadKey};
    use nym_lp::message::{ApplicationData, LpMessage};
    use nym_lp::packet::{LpHeader, LpPacket};
    use nym_lp::peer::LpLocalPeer;
    use nym_lp::SessionsMock;
    use std::sync::Arc;
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

        let id_keys = Arc::new(ed25519::KeyPair::new(&mut OsRng));
        let x_keys = Arc::new(id_keys.to_x25519());

        let lp_peer = LpLocalPeer::new(id_keys, x_keys.clone()).with_kem_psq_key(x_keys);

        LpHandlerState {
            lp_config,
            ecash_verifier: Arc::new(ecash_verifier)
                as Arc<dyn nym_credential_verification::ecash::traits::EcashManager + Send + Sync>,
            storage,
            local_lp_peer: lp_peer,
            metrics: nym_node_metrics::NymNodeMetrics::default(),
            active_clients_store: ActiveClientsStore::new(),
            outbound_mix_sender: mix_sender,
            session_states: Arc::new(dashmap::DashMap::new()),
            forward_semaphore,
            peer_registrator: None,
        }
    }

    fn add_dummy_lp_state(handler: &mut LpConnectionHandler, session: LpSession) {
        let id = session.id();
        let state_machine = LpStateMachine::new(session);
        handler.bound_receiver_idx = Some(id);

        handler
            .state
            .session_states
            .insert(id, TimestampedState::new(state_machine));
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
        outer_aead_key: &OuterAeadKey,
    ) -> Result<LpPacket, std::io::Error> {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let packet_len = u32::from_be_bytes(len_buf) as usize;

        // Read packet data
        let mut packet_buf = vec![0u8; packet_len];
        stream.read_exact(&mut packet_buf).await?;

        // Parse packet
        parse_lp_packet(&packet_buf, Some(outer_aead_key))
            .map_err(|e| std::io::Error::other(e.to_string()))
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
                reserved: [0u8; 3],
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

        let receiver_idx = 99;
        let sessions = SessionsMock::mock_post_handshake(receiver_idx);
        let init = sessions.initiator;
        let resp = sessions.responder;

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            add_dummy_lp_state(&mut handler, resp);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx,
                    counter: 5,
                },
                LpMessage::Busy,
            );
            handler.send_lp_packet(packet).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Wait for server to send
        server_task.await.unwrap().unwrap();

        // Client should receive it correctly
        let received = read_lp_packet_from_stream(&mut client_stream, init.outer_aead_key())
            .await
            .unwrap();
        assert_eq!(received.header().receiver_idx, receiver_idx);
        assert_eq!(received.header().counter, 5);
    }

    #[tokio::test]
    async fn test_send_receive_encrypted_data_message() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let receiver_idx = 200;
        let sessions = SessionsMock::mock_post_handshake(receiver_idx);
        let init = sessions.initiator;
        let resp = sessions.responder;

        let encrypted_payload = vec![42u8; 256];
        let expected_payload = encrypted_payload.clone();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            add_dummy_lp_state(&mut handler, resp);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx,
                    counter: 20,
                },
                LpMessage::ApplicationData(ApplicationData(encrypted_payload)),
            );
            handler.send_lp_packet(packet).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();
        server_task.await.unwrap().unwrap();

        let received = read_lp_packet_from_stream(&mut client_stream, init.outer_aead_key())
            .await
            .unwrap();
        assert_eq!(received.header().receiver_idx, 200);
        assert_eq!(received.header().counter, 20);
        match received.message() {
            LpMessage::ApplicationData(data) => {
                assert_eq!(data, &ApplicationData(expected_payload))
            }
            _ => panic!("Expected EncryptedData message"),
        }
    }
}
