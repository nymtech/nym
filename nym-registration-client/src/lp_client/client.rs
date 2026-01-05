// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) registration client for direct gateway connections.

use super::config::LpConfig;
use super::error::{LpClientError, Result};
use bytes::BytesMut;
use nym_bandwidth_controller::{BandwidthTicketProvider, DEFAULT_TICKETS_TO_SPEND};
use nym_credentials_interface::{CredentialSpendingData, TicketType};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::LpPacket;
use nym_lp::codec::{OuterAeadKey, parse_lp_packet, serialize_lp_packet};
use nym_lp::message::ForwardPacketData;
use nym_lp::state_machine::{LpAction, LpInput, LpStateMachine};
use nym_registration_common::{GatewayData, LpRegistrationRequest, LpRegistrationResponse};
use nym_wireguard_types::PeerPublicKey;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// LP (Lewes Protocol) registration client for direct gateway connections.
///
/// This client uses a persistent TCP connection model where a single TCP
/// connection is used for the entire handshake and registration flow.
/// The connection is opened on first use and closed after registration.
///
/// # Example Flow
/// ```ignore
/// let mut client = LpRegistrationClient::new(...);
/// client.perform_handshake().await?;            // Noise handshake (single connection)
/// let gateway_data = client.register(...).await?;  // Registration (same connection)
/// // Connection automatically closes after registration
/// ```
pub struct LpRegistrationClient {
    /// Client's Ed25519 identity keypair (used for PSQ authentication and X25519 derivation).
    local_ed25519_keypair: Arc<ed25519::KeyPair>,

    /// Gateway's Ed25519 public key (from directory/discovery).
    gateway_ed25519_public_key: ed25519::PublicKey,

    /// Gateway LP listener address (host:port, e.g., "1.1.1.1:41264").
    gateway_lp_address: SocketAddr,

    /// LP state machine for managing connection lifecycle.
    /// Created during handshake initiation. Persists across packet-per-connection calls.
    state_machine: Option<LpStateMachine>,

    /// Client's IP address for registration metadata.
    client_ip: IpAddr,

    /// Configuration for timeouts and TCP parameters.
    config: LpConfig,

    /// Persistent TCP stream for the connection.
    /// Opened on first use, closed after registration.
    stream: Option<TcpStream>,
}

impl LpRegistrationClient {
    /// Creates a new LP registration client.
    ///
    /// # Arguments
    /// * `local_ed25519_keypair` - Client's Ed25519 identity keypair (for PSQ auth and X25519 derivation)
    /// * `gateway_ed25519_public_key` - Gateway's Ed25519 public key (from directory/discovery)
    /// * `gateway_lp_address` - Gateway's LP listener socket address
    /// * `client_ip` - Client IP address for registration
    /// * `config` - Configuration for timeouts and TCP parameters (use `LpConfig::default()`)
    ///
    /// # Note
    /// This creates the client. Call `perform_handshake()` to establish the LP session.
    /// Each packet exchange opens a new TCP connection (packet-per-connection model).
    pub fn new(
        local_ed25519_keypair: Arc<ed25519::KeyPair>,
        gateway_ed25519_public_key: ed25519::PublicKey,
        gateway_lp_address: SocketAddr,
        client_ip: IpAddr,
        config: LpConfig,
    ) -> Self {
        Self {
            local_ed25519_keypair,
            gateway_ed25519_public_key,
            gateway_lp_address,
            state_machine: None,
            client_ip,
            config,
            stream: None,
        }
    }

    /// Creates a new LP registration client with default configuration.
    ///
    /// # Arguments
    /// * `local_ed25519_keypair` - Client's Ed25519 identity keypair
    /// * `gateway_ed25519_public_key` - Gateway's Ed25519 public key
    /// * `gateway_lp_address` - Gateway's LP listener socket address
    /// * `client_ip` - Client IP address for registration
    ///
    /// Uses default config (LpConfig::default()) with sane timeout and TCP parameters.
    /// PSK is derived automatically during handshake inside the state machine.
    /// For custom config, use `new()` directly.
    pub fn new_with_default_psk(
        local_ed25519_keypair: Arc<ed25519::KeyPair>,
        gateway_ed25519_public_key: ed25519::PublicKey,
        gateway_lp_address: SocketAddr,
        client_ip: IpAddr,
    ) -> Self {
        Self::new(
            local_ed25519_keypair,
            gateway_ed25519_public_key,
            gateway_lp_address,
            client_ip,
            LpConfig::default(),
        )
    }

    /// Returns whether the client has completed the handshake and is ready for registration.
    pub fn is_handshake_complete(&self) -> bool {
        self.state_machine
            .as_ref()
            .and_then(|sm| sm.session().ok())
            .map(|s| s.is_handshake_complete())
            .unwrap_or(false)
    }

    /// Returns the gateway LP address this client is configured for.
    pub fn gateway_address(&self) -> SocketAddr {
        self.gateway_lp_address
    }

    /// Returns the client's IP address.
    pub fn client_ip(&self) -> IpAddr {
        self.client_ip
    }

    // -------------------------------------------------------------------------
    // Persistent connection management
    // -------------------------------------------------------------------------

    /// Ensures a TCP connection is established.
    ///
    /// Opens a new connection to the gateway if one doesn't exist.
    /// If a connection already exists, returns immediately.
    ///
    /// # Errors
    /// Returns an error if connection fails or times out.
    async fn ensure_connected(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Ok(());
        }

        tracing::debug!(
            "Opening persistent connection to {}",
            self.gateway_lp_address
        );

        let stream = tokio::time::timeout(
            self.config.connect_timeout,
            TcpStream::connect(self.gateway_lp_address),
        )
        .await
        .map_err(|_| LpClientError::TcpConnection {
            address: self.gateway_lp_address.to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("Connection timeout after {:?}", self.config.connect_timeout),
            ),
        })?
        .map_err(|source| LpClientError::TcpConnection {
            address: self.gateway_lp_address.to_string(),
            source,
        })?;

        // Set TCP_NODELAY for low latency
        stream
            .set_nodelay(self.config.tcp_nodelay)
            .map_err(|source| LpClientError::TcpConnection {
                address: self.gateway_lp_address.to_string(),
                source,
            })?;

        self.stream = Some(stream);
        tracing::debug!(
            "Persistent connection established to {}",
            self.gateway_lp_address
        );
        Ok(())
    }

    /// Sends an LP packet on the persistent stream.
    ///
    /// # Arguments
    /// * `packet` - The LP packet to send
    /// * `outer_key` - Optional outer AEAD key for encryption
    ///
    /// # Errors
    /// Returns an error if not connected or if send fails.
    async fn send_packet(
        &mut self,
        packet: &LpPacket,
        outer_key: Option<&OuterAeadKey>,
    ) -> Result<()> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| LpClientError::Transport("Cannot send: not connected".to_string()))?;

        Self::send_packet_with_key(stream, packet, outer_key).await
    }

    /// Receives an LP packet from the persistent stream.
    ///
    /// # Arguments
    /// * `outer_key` - Optional outer AEAD key for decryption
    ///
    /// # Errors
    /// Returns an error if not connected or if receive fails.
    async fn receive_packet(&mut self, outer_key: Option<&OuterAeadKey>) -> Result<LpPacket> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| LpClientError::Transport("Cannot receive: not connected".to_string()))?;

        Self::receive_packet_with_key(stream, outer_key).await
    }

    /// Closes the persistent connection.
    ///
    /// This drops the TCP stream, signaling EOF to the gateway.
    /// Safe to call even if not connected.
    ///
    /// # Connection Lifecycle
    /// The connection stays open after handshake and registration to support
    /// follow-up operations like `send_forward_packet()`. Callers should:
    /// - For direct registration: call `close()` after `register()` returns
    /// - For nested sessions: call `close()` after all forwarding is complete
    ///
    /// The connection will also close automatically when the client is dropped.
    pub fn close(&mut self) {
        if self.stream.take().is_some() {
            tracing::debug!(
                "Closed persistent connection to {}",
                self.gateway_lp_address
            );
        }
    }

    // -------------------------------------------------------------------------
    // Handshake
    // -------------------------------------------------------------------------

    /// Performs the LP Noise protocol handshake with the gateway.
    ///
    /// This establishes a secure encrypted session using the Noise protocol.
    /// Uses a persistent TCP connection for all handshake messages.
    ///
    /// # Errors
    /// Returns an error if:
    /// - State machine creation fails
    /// - Handshake protocol fails
    /// - Network communication fails
    /// - Handshake times out (see LpConfig::handshake_timeout)
    ///
    /// # Implementation
    /// This implements the Noise protocol handshake as the initiator:
    /// 1. Opens persistent TCP connection (if not already connected)
    /// 2. Sends ClientHello, receives Ack
    /// 3. Creates LP state machine with client as initiator
    /// 4. Exchanges handshake messages on the same connection
    /// 5. Stores the established session in the state machine
    ///
    /// The connection remains open after handshake for registration/forwarding.
    pub async fn perform_handshake(&mut self) -> Result<()> {
        // Apply handshake timeout (nym-102)
        let result = tokio::time::timeout(
            self.config.handshake_timeout,
            self.perform_handshake_inner(),
        )
        .await;

        // Clean up connection on any error to prevent state machine inconsistency
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                self.close();
                Err(e)
            }
            Err(_) => {
                self.close();
                Err(LpClientError::Transport(format!(
                    "Handshake timeout after {:?}",
                    self.config.handshake_timeout
                )))
            }
        }
    }

    /// Internal handshake implementation without timeout.
    ///
    /// Uses a persistent TCP connection: all handshake packets are sent and
    /// received on the same connection. The connection remains open for
    /// registration/forwarding after handshake completes.
    async fn perform_handshake_inner(&mut self) -> Result<()> {
        tracing::debug!("Starting LP handshake as initiator (persistent connection)");

        // Ensure we have a TCP connection
        self.ensure_connected().await?;

        // Step 1: Derive X25519 keys from Ed25519 for Noise protocol (internal to ClientHello)
        // The Ed25519 keys are used for PSQ authentication and also converted to X25519
        let client_x25519_public = self
            .local_ed25519_keypair
            .public_key()
            .to_x25519()
            .map_err(|e| {
                LpClientError::Crypto(format!("Failed to derive X25519 public key: {e}"))
            })?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        // Step 2: Generate ClientHelloData with fresh salt and both public keys
        let client_hello_data = nym_lp::ClientHelloData::new_with_fresh_salt(
            client_x25519_public.to_bytes(),
            self.local_ed25519_keypair.public_key().to_bytes(),
            timestamp,
        );
        let salt = client_hello_data.salt;
        let receiver_index = client_hello_data.receiver_index;

        tracing::trace!(
            "Generated ClientHello with timestamp: {}, receiver_index: {}",
            client_hello_data.extract_timestamp(),
            receiver_index
        );

        // Step 3: Send ClientHello and receive Ack (persistent connection)
        let client_hello_header = nym_lp::packet::LpHeader::new(
            nym_lp::BOOTSTRAP_RECEIVER_IDX, // session_id not yet established
            0,                              // counter starts at 0
        );
        let client_hello_packet = nym_lp::LpPacket::new(
            client_hello_header,
            nym_lp::LpMessage::ClientHello(client_hello_data),
        );

        // Send ClientHello (no outer key - before PSK)
        self.send_packet(&client_hello_packet, None).await?;
        // Receive Ack (no outer key - before PSK)
        let ack_response = self.receive_packet(None).await?;

        // Verify we received Ack
        match ack_response.message() {
            nym_lp::LpMessage::Ack => {
                tracing::debug!("Received Ack for ClientHello");
            }
            other => {
                return Err(LpClientError::Transport(format!(
                    "Expected Ack for ClientHello, got: {:?}",
                    other
                )));
            }
        }

        // Step 4: Create state machine as initiator with Ed25519 keys
        // PSK derivation happens internally in the state machine constructor
        let mut state_machine = LpStateMachine::new(
            receiver_index,
            true, // is_initiator
            (
                self.local_ed25519_keypair.private_key(),
                self.local_ed25519_keypair.public_key(),
            ),
            &self.gateway_ed25519_public_key,
            &salt,
        )?;

        // Step 5: Start handshake - get first packet to send (KKT request)
        let mut pending_packet: Option<nym_lp::LpPacket> = None;
        if let Some(action) = state_machine.process_input(LpInput::StartHandshake) {
            match action? {
                LpAction::SendPacket(packet) => {
                    pending_packet = Some(packet);
                }
                other => {
                    return Err(LpClientError::Transport(format!(
                        "Unexpected action at handshake start: {:?}",
                        other
                    )));
                }
            }
        }

        // Step 6: Handshake loop - all packets on persistent connection
        loop {
            // Send pending packet if we have one
            if let Some(packet) = pending_packet.take() {
                // Get outer keys from session:
                // - send_key: outer_aead_key_for_sending() returns None until PSQ complete
                // - recv_key: outer_aead_key() returns key as soon as PSK is derived
                let send_key = state_machine
                    .session()
                    .ok()
                    .and_then(|s| s.outer_aead_key_for_sending());
                let recv_key = state_machine
                    .session()
                    .ok()
                    .and_then(|s| s.outer_aead_key());

                tracing::trace!(
                    "Sending handshake packet (send_key={}, recv_key={})",
                    send_key.is_some(),
                    recv_key.is_some()
                );
                self.send_packet(&packet, send_key.as_ref()).await?;
                let response = self.receive_packet(recv_key.as_ref()).await?;
                tracing::trace!("Received handshake response");

                // Process the received packet
                if let Some(action) = state_machine.process_input(LpInput::ReceivePacket(response))
                {
                    match action? {
                        LpAction::SendPacket(response_packet) => {
                            // Queue the response packet to send on next iteration
                            pending_packet = Some(response_packet);

                            // Check if handshake completed after queueing this packet
                            if state_machine.session()?.is_handshake_complete() {
                                // Send the final packet before breaking
                                if let Some(final_packet) = pending_packet.take() {
                                    let send_key = state_machine
                                        .session()
                                        .ok()
                                        .and_then(|s| s.outer_aead_key_for_sending());
                                    let recv_key = state_machine
                                        .session()
                                        .ok()
                                        .and_then(|s| s.outer_aead_key());
                                    tracing::trace!("Sending final handshake packet");
                                    self.send_packet(&final_packet, send_key.as_ref()).await?;
                                    let ack_response =
                                        self.receive_packet(recv_key.as_ref()).await?;

                                    // Validate Ack response
                                    match ack_response.message() {
                                        nym_lp::LpMessage::Ack => {
                                            tracing::debug!(
                                                "Received Ack for final handshake packet"
                                            );
                                        }
                                        other => {
                                            return Err(LpClientError::Transport(format!(
                                                "Expected Ack for final handshake packet, got: {:?}",
                                                other
                                            )));
                                        }
                                    }
                                }
                                tracing::info!("LP handshake completed after sending final packet");
                                break;
                            }
                        }
                        LpAction::HandshakeComplete => {
                            tracing::info!("LP handshake completed successfully");
                            break;
                        }
                        LpAction::KKTComplete => {
                            tracing::info!("KKT exchange completed, starting Noise handshake");
                            // After KKT completes, initiator must send first Noise handshake message
                            let noise_msg = state_machine
                                .session()?
                                .prepare_handshake_message()
                                .ok_or_else(|| {
                                LpClientError::transport("No handshake message available after KKT")
                            })??;
                            let noise_packet = state_machine.session()?.next_packet(noise_msg)?;
                            pending_packet = Some(noise_packet);
                        }
                        other => {
                            tracing::trace!("Received action during handshake: {:?}", other);
                        }
                    }
                }
            } else {
                // No pending packet and not complete - something is wrong
                return Err(LpClientError::transport(
                    "Handshake stalled: no packet to send",
                ));
            }
        }

        // Store the state machine (with established session) for later use
        self.state_machine = Some(state_machine);
        Ok(())
    }

    /// Opens a TCP connection, sends one packet, receives one response, closes.
    ///
    /// This implements the packet-per-connection model where each LP packet
    /// exchange uses its own TCP connection. The connection is closed when
    /// this method returns (stream dropped).
    ///
    /// # Arguments
    /// * `address` - Gateway LP listener address
    /// * `packet` - The LP packet to send
    /// * `outer_key` - Optional outer AEAD key (None before PSK, Some after)
    /// * `config` - Configuration for timeouts and TCP parameters
    ///
    /// # Errors
    /// Returns an error if connection, send, or receive fails.
    ///
    /// # Outer AEAD Keys
    ///
    /// Send and receive use separate keys because during the PSQ handshake:
    /// - Initiator derives PSK when preparing msg 1, but must send it cleartext
    ///   (responder hasn't derived PSK yet)
    /// - Responder sends msg 2 encrypted (both have PSK now)
    /// - Initiator can decrypt msg 2 (has had PSK since preparing msg 1)
    ///
    /// Use `outer_aead_key_for_sending()` for `send_key` (gates on PSQ completion)
    /// and `outer_aead_key()` for `recv_key` (available as soon as PSK derived).
    ///
    /// # Note
    /// This method is kept for reference but is no longer used. The persistent
    /// connection model uses `send_packet()` and `receive_packet()` instead.
    #[allow(dead_code)]
    async fn connect_send_receive(
        address: SocketAddr,
        packet: &LpPacket,
        send_key: Option<&OuterAeadKey>,
        recv_key: Option<&OuterAeadKey>,
        config: &LpConfig,
    ) -> Result<LpPacket> {
        // 1. Connect with timeout
        let mut stream = tokio::time::timeout(config.connect_timeout, TcpStream::connect(address))
            .await
            .map_err(|_| LpClientError::TcpConnection {
                address: address.to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Connection timeout after {:?}", config.connect_timeout),
                ),
            })?
            .map_err(|source| LpClientError::TcpConnection {
                address: address.to_string(),
                source,
            })?;

        // 2. Set TCP_NODELAY
        stream
            .set_nodelay(config.tcp_nodelay)
            .map_err(|source| LpClientError::TcpConnection {
                address: address.to_string(),
                source,
            })?;

        // 3. Send packet with send_key
        Self::send_packet_with_key(&mut stream, packet, send_key).await?;

        // 4. Receive response with recv_key
        let response = Self::receive_packet_with_key(&mut stream, recv_key).await?;

        // Connection drops when stream goes out of scope
        Ok(response)
    }

    /// Sends an LP packet over a TCP stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Arguments
    /// * `stream` - TCP stream to send on
    /// * `packet` - The LP packet to send
    /// * `outer_key` - Optional outer AEAD key for encryption
    ///
    /// # Errors
    /// Returns an error if serialization or network transmission fails.
    async fn send_packet_with_key(
        stream: &mut TcpStream,
        packet: &LpPacket,
        outer_key: Option<&OuterAeadKey>,
    ) -> Result<()> {
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(packet, &mut packet_buf, outer_key)
            .map_err(|e| LpClientError::Transport(format!("Failed to serialize packet: {e}")))?;

        // Send 4-byte length prefix (u32 big-endian)
        let len = packet_buf.len() as u32;
        stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to send packet length: {e}")))?;

        // Send the actual packet data
        stream
            .write_all(&packet_buf)
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to send packet data: {e}")))?;

        // Flush to ensure data is sent immediately
        stream
            .flush()
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to flush stream: {e}")))?;

        tracing::trace!(
            "Sent LP packet ({} bytes + 4 byte header)",
            packet_buf.len()
        );
        Ok(())
    }

    /// Receives an LP packet from a TCP stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Arguments
    /// * `stream` - TCP stream to receive from
    /// * `outer_key` - Optional outer AEAD key for decryption
    ///
    /// # Errors
    /// Returns an error if:
    /// - Network read fails
    /// - Packet size exceeds maximum (64KB)
    /// - Packet parsing/decryption fails
    async fn receive_packet_with_key(
        stream: &mut TcpStream,
        outer_key: Option<&OuterAeadKey>,
    ) -> Result<LpPacket> {
        // Read 4-byte length prefix (u32 big-endian)
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to read packet length: {e}")))?;

        let packet_len = u32::from_be_bytes(len_buf) as usize;

        // Sanity check to prevent huge allocations
        const MAX_PACKET_SIZE: usize = 65536; // 64KB max
        if packet_len > MAX_PACKET_SIZE {
            return Err(LpClientError::Transport(format!(
                "Packet size {packet_len} exceeds maximum {MAX_PACKET_SIZE}",
            )));
        }

        // Read the actual packet data
        let mut packet_buf = vec![0u8; packet_len];
        stream
            .read_exact(&mut packet_buf)
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to read packet data: {e}")))?;

        let packet = parse_lp_packet(&packet_buf, outer_key)
            .map_err(|e| LpClientError::Transport(format!("Failed to parse packet: {e}")))?;

        tracing::trace!("Received LP packet ({} bytes + 4 byte header)", packet_len);
        Ok(packet)
    }

    /// Sends registration request and receives response in a single operation.
    ///
    /// This is the primary registration method. It acquires a bandwidth credential,
    /// sends the registration request, and receives the response using the
    /// packet-per-connection model.
    ///
    /// # Arguments
    /// * `wg_keypair` - Client's WireGuard x25519 keypair
    /// * `gateway_identity` - Gateway's ed25519 identity for credential verification
    /// * `bandwidth_controller` - Provider for bandwidth credentials
    /// * `ticket_type` - Type of bandwidth ticket to use
    ///
    /// # Returns
    /// * `Ok(GatewayData)` - Gateway configuration data on successful registration
    ///
    /// # Errors
    /// Returns an error if:
    /// - Handshake has not been completed
    /// - Credential acquisition fails
    /// - Request serialization/encryption fails
    /// - Network communication fails
    /// - Gateway rejected the registration
    /// - Response times out (see LpConfig::registration_timeout)
    pub async fn register(
        &mut self,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
    ) -> Result<GatewayData> {
        tracing::debug!("Acquiring bandwidth credential for registration");

        // Get bandwidth credential from controller
        let credential = bandwidth_controller
            .get_ecash_ticket(ticket_type, *gateway_identity, DEFAULT_TICKETS_TO_SPEND)
            .await
            .map_err(|e| {
                LpClientError::SendRegistrationRequest(format!(
                    "Failed to acquire bandwidth credential: {}",
                    e
                ))
            })?
            .data;

        self.register_with_credential(wg_keypair, credential, ticket_type)
            .await
    }

    /// Sends registration request with a pre-generated credential.
    ///
    /// This is useful for testing with mock ecash credentials.
    /// Uses the persistent TCP connection established during handshake.
    ///
    /// # Arguments
    /// * `wg_keypair` - Client's WireGuard x25519 keypair
    /// * `credential` - Pre-generated bandwidth credential
    /// * `ticket_type` - Type of bandwidth ticket
    ///
    /// # Returns
    /// * `Ok(GatewayData)` - Gateway configuration data on successful registration
    ///
    /// # Connection Lifecycle
    /// The connection stays open after registration to support `send_forward_packet()`.
    /// Callers should call `close()` when done with all operations.
    ///
    /// # Panics / Errors
    /// Returns error if handshake not completed or if connection was closed.
    pub async fn register_with_credential(
        &mut self,
        wg_keypair: &x25519::KeyPair,
        credential: CredentialSpendingData,
        ticket_type: TicketType,
    ) -> Result<GatewayData> {
        tracing::debug!("Sending registration request (persistent connection)");

        // 1. Build registration request
        let wg_public_key = PeerPublicKey::new(wg_keypair.public_key().to_bytes().into());
        let request =
            LpRegistrationRequest::new_dvpn(wg_public_key, credential, ticket_type, self.client_ip);

        tracing::trace!("Built registration request: {:?}", request);

        // 2. Serialize the request
        let request_bytes = bincode::serialize(&request).map_err(|e| {
            LpClientError::SendRegistrationRequest(format!("Failed to serialize request: {e}"))
        })?;

        tracing::debug!(
            "Sending registration request ({} bytes)",
            request_bytes.len()
        );

        // 3. Encrypt and prepare packet via state machine (scoped borrow)
        let (request_packet, send_key, recv_key) = {
            let state_machine = self.state_machine.as_mut().ok_or_else(|| {
                LpClientError::transport("Cannot register: handshake not completed")
            })?;

            let action = state_machine
                .process_input(LpInput::SendData(request_bytes))
                .ok_or_else(|| LpClientError::transport("State machine returned no action"))?
                .map_err(|e| {
                    LpClientError::SendRegistrationRequest(format!(
                        "Failed to encrypt registration request: {e}",
                    ))
                })?;

            let request_packet = match action {
                LpAction::SendPacket(packet) => packet,
                other => {
                    return Err(LpClientError::Transport(format!(
                        "Unexpected action when sending registration data: {other:?}",
                    )));
                }
            };

            // Get outer keys from session
            let send_key = state_machine
                .session()
                .ok()
                .and_then(|s| s.outer_aead_key_for_sending());
            let recv_key = state_machine
                .session()
                .ok()
                .and_then(|s| s.outer_aead_key());

            (request_packet, send_key, recv_key)
        }; // state_machine borrow ends here

        // 4. Send request and receive response on persistent connection with timeout
        let response_packet = tokio::time::timeout(self.config.registration_timeout, async {
            self.send_packet(&request_packet, send_key.as_ref()).await?;
            self.receive_packet(recv_key.as_ref()).await
        })
        .await
        .map_err(|_| {
            LpClientError::ReceiveRegistrationResponse(format!(
                "Registration timeout after {:?}",
                self.config.registration_timeout
            ))
        })??;

        tracing::trace!("Received registration response packet");

        // 5. Decrypt via state machine (re-borrow)
        let state_machine = self
            .state_machine
            .as_mut()
            .ok_or_else(|| LpClientError::transport("State machine disappeared unexpectedly"))?;
        let action = state_machine
            .process_input(LpInput::ReceivePacket(response_packet))
            .ok_or_else(|| LpClientError::transport("State machine returned no action"))?
            .map_err(|e| {
                LpClientError::ReceiveRegistrationResponse(format!(
                    "Failed to decrypt registration response: {e}",
                ))
            })?;

        // 7. Extract decrypted data
        let response_data = match action {
            LpAction::DeliverData(data) => data,
            other => {
                return Err(LpClientError::Transport(format!(
                    "Unexpected action when receiving registration response: {other:?}"
                )));
            }
        };

        // 8. Deserialize the response
        let response: LpRegistrationResponse =
            bincode::deserialize(&response_data).map_err(|e| {
                LpClientError::ReceiveRegistrationResponse(format!(
                    "Failed to deserialize registration response: {e}",
                ))
            })?;

        tracing::debug!(
            "Received registration response: success={}",
            response.success,
        );

        // 9. Validate and extract GatewayData
        if !response.success {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            tracing::warn!("Gateway rejected registration: {error_msg}");
            return Err(LpClientError::RegistrationRejected { reason: error_msg });
        }

        let gateway_data = response.gateway_data.ok_or_else(|| {
            LpClientError::ReceiveRegistrationResponse(
                "Gateway response missing gateway_data despite success=true".to_string(),
            )
        })?;

        tracing::info!(
            "LP registration successful! Allocated bandwidth: {} bytes",
            response.allocated_bandwidth
        );

        Ok(gateway_data)
    }

    /// Register with automatic retry on network failure.
    ///
    /// This method:
    /// 1. Acquires credential ONCE
    /// 2. Performs handshake if not already connected
    /// 3. Attempts registration
    /// 4. On network failure, re-establishes connection and retries with same credential
    /// 5. Gateway idempotency ensures no double-spend even if credential was processed
    ///
    /// Use this method for resilient registration on unreliable networks (e.g., train
    /// through tunnel). The gateway's idempotent registration check ensures that if
    /// a registration succeeds but the response is lost, retrying with the same WG key
    /// will return the cached result instead of spending a new credential.
    ///
    /// # Arguments
    /// * `wg_keypair` - Client's WireGuard x25519 keypair (same key used for all retries)
    /// * `gateway_identity` - Gateway's ed25519 identity for credential verification
    /// * `bandwidth_controller` - Provider for bandwidth credentials
    /// * `ticket_type` - Type of bandwidth ticket to use
    /// * `max_retries` - Maximum number of retry attempts after initial failure
    ///
    /// # Returns
    /// * `Ok(GatewayData)` - Gateway configuration data on successful registration
    ///
    /// # Errors
    /// Returns an error if all retry attempts fail.
    ///
    /// # Note
    /// Unlike `register()`, this method handles the full flow including handshake.
    /// Do NOT call `perform_handshake()` before this method.
    pub async fn register_with_retry(
        &mut self,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
        max_retries: u32,
    ) -> Result<GatewayData> {
        tracing::debug!("Starting resilient registration (max_retries={max_retries})",);

        // Acquire credential ONCE before any attempts
        let credential = bandwidth_controller
            .get_ecash_ticket(ticket_type, *gateway_identity, DEFAULT_TICKETS_TO_SPEND)
            .await
            .map_err(|e| {
                LpClientError::SendRegistrationRequest(format!(
                    "Failed to acquire bandwidth credential: {e}",
                ))
            })?
            .data;

        let mut last_error = None;
        for attempt in 0..=max_retries {
            if attempt > 0 {
                // Exponential backoff with jitter: 100ms, 200ms, 400ms, 800ms, 1600ms (capped)
                let base_delay_ms = 100u64 * (1 << attempt.min(4));
                let jitter_ms = rand::random::<u64>() % (base_delay_ms / 4 + 1);
                let delay = std::time::Duration::from_millis(base_delay_ms + jitter_ms);
                tracing::info!(
                    "Retrying registration (attempt {}) after {:?}",
                    attempt + 1,
                    delay
                );
                tokio::time::sleep(delay).await;
            }

            // Ensure fresh connection and handshake for each attempt
            // (On retry, the old connection/session may be dead)
            if self.stream.is_none() || attempt > 0 {
                // Clear any stale state before re-handshaking
                self.close();
                self.state_machine = None;

                if let Err(e) = self.perform_handshake().await {
                    tracing::warn!("Handshake failed on attempt {}: {e}", attempt + 1);
                    last_error = Some(e);
                    continue;
                }
            }

            match self
                .register_with_credential(wg_keypair, credential.clone(), ticket_type)
                .await
            {
                Ok(data) => {
                    if attempt > 0 {
                        tracing::info!("Registration succeeded on retry attempt {}", attempt + 1);
                    }
                    return Ok(data);
                }
                Err(e) => {
                    tracing::warn!("Registration attempt {} failed: {e}", attempt + 1);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| LpClientError::transport("Registration failed after all retries")))
    }

    /// Sends a ForwardPacket message to the entry gateway for forwarding to the exit gateway.
    ///
    /// This method constructs a ForwardPacket containing the target gateway's identity,
    /// address, and the inner LP packet bytes, encrypts it through the outer session
    /// (client-entry), and receives the response from the exit gateway via the entry gateway.
    ///
    /// Uses the persistent TCP connection established during handshake.
    /// Multiple forward packets can be sent on the same connection.
    ///
    /// # Arguments
    /// * `target_identity` - Target gateway's Ed25519 identity (32 bytes)
    /// * `target_address` - Target gateway's LP address (e.g., "1.1.1.1:41264")
    /// * `inner_packet_bytes` - Complete inner LP packet bytes to forward to exit gateway
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Decrypted response bytes from the exit gateway
    ///
    /// # Errors
    /// Returns an error if:
    /// - Handshake has not been completed
    /// - Serialization fails
    /// - Encryption or network transmission fails
    /// - Response decryption fails
    ///
    /// # Example Flow
    /// ```ignore
    /// // Construct inner packet for exit gateway (ClientHello, handshake, etc.)
    /// let inner_packet = LpPacket::new(...);
    /// let inner_bytes = serialize_lp_packet(&inner_packet, &mut BytesMut::new())?;
    ///
    /// // Forward through entry gateway
    /// let response_bytes = client.send_forward_packet(
    ///     exit_identity,
    ///     "2.2.2.2:41264".to_string(),
    ///     inner_bytes.to_vec(),
    /// ).await?;
    /// ```
    pub async fn send_forward_packet(
        &mut self,
        target_identity: [u8; 32],
        target_address: String,
        inner_packet_bytes: Vec<u8>,
    ) -> Result<Vec<u8>> {
        tracing::debug!(
            "Sending ForwardPacket to {} ({} inner bytes, persistent connection)",
            target_address,
            inner_packet_bytes.len()
        );

        // 1. Construct ForwardPacketData
        let forward_data = ForwardPacketData {
            target_gateway_identity: target_identity,
            target_lp_address: target_address.clone(),
            inner_packet_bytes,
        };

        // 2. Serialize the ForwardPacketData
        let forward_data_bytes = bincode::serialize(&forward_data).map_err(|e| {
            LpClientError::Transport(format!("Failed to serialize ForwardPacketData: {e}"))
        })?;

        tracing::trace!(
            "Serialized ForwardPacketData ({} bytes)",
            forward_data_bytes.len()
        );

        // 3. Encrypt and prepare packet via state machine (scoped borrow)
        let (forward_packet, send_key, recv_key) = {
            let state_machine = self.state_machine.as_mut().ok_or_else(|| {
                LpClientError::transport("Cannot send forward packet: handshake not completed")
            })?;

            let action = state_machine
                .process_input(LpInput::SendData(forward_data_bytes))
                .ok_or_else(|| LpClientError::transport("State machine returned no action"))?
                .map_err(|e| {
                    LpClientError::Transport(format!("Failed to encrypt ForwardPacket: {e}"))
                })?;

            let forward_packet = match action {
                LpAction::SendPacket(packet) => packet,
                other => {
                    return Err(LpClientError::Transport(format!(
                        "Unexpected action when sending ForwardPacket: {:?}",
                        other
                    )));
                }
            };

            // Get outer keys from session
            let send_key = state_machine
                .session()
                .ok()
                .and_then(|s| s.outer_aead_key_for_sending());
            let recv_key = state_machine
                .session()
                .ok()
                .and_then(|s| s.outer_aead_key());

            (forward_packet, send_key, recv_key)
        }; // state_machine borrow ends here

        // 4. Send and receive on persistent connection with timeout
        let response_packet = tokio::time::timeout(self.config.forward_timeout, async {
            self.send_packet(&forward_packet, send_key.as_ref()).await?;
            self.receive_packet(recv_key.as_ref()).await
        })
        .await
        .map_err(|_| {
            LpClientError::Transport(format!(
                "Forward packet timeout after {:?}",
                self.config.forward_timeout
            ))
        })??;
        tracing::trace!("Received response packet from entry gateway");

        // 5. Decrypt via state machine (re-borrow)
        let state_machine = self
            .state_machine
            .as_mut()
            .ok_or_else(|| LpClientError::transport("State machine disappeared unexpectedly"))?;
        let action = state_machine
            .process_input(LpInput::ReceivePacket(response_packet))
            .ok_or_else(|| LpClientError::transport("State machine returned no action"))?
            .map_err(|e| {
                LpClientError::Transport(format!("Failed to decrypt forward response: {e}"))
            })?;

        // 7. Extract decrypted response data
        let response_data = match action {
            LpAction::DeliverData(data) => data,
            other => {
                return Err(LpClientError::Transport(format!(
                    "Unexpected action when receiving forward response: {:?}",
                    other
                )));
            }
        };

        tracing::debug!(
            "Successfully received forward response from {} ({} bytes)",
            target_address,
            response_data.len()
        );

        Ok(response_data.to_vec())
    }

    /// Wrap data in an LP packet for UDP transmission to the data plane (port 51264).
    ///
    /// This method encrypts the provided data using the established LP session
    /// and returns serialized LP packet bytes ready to send over UDP.
    ///
    /// # Prerequisites
    /// - Handshake must be completed (`perform_handshake()`)
    ///
    /// # Arguments
    /// * `data` - Raw application data to wrap (e.g., Sphinx packet bytes)
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Serialized LP packet bytes (outer header + encrypted payload)
    ///
    /// # Wire Format
    /// The returned bytes are in LP wire format:
    /// - Outer header (12B): receiver_idx(4) + counter(8) - always cleartext
    /// - Encrypted payload: proto(1) + reserved(3) + msg_type(4) + content + tag(16)
    ///
    /// # Usage
    /// After LP handshake, wrap Sphinx packets before sending to gateway's LP data port:
    /// ```ignore
    /// client.perform_handshake().await?;
    /// let sphinx_bytes = build_sphinx_packet(...);
    /// let lp_bytes = client.wrap_data(&sphinx_bytes)?;
    /// socket.send_to(&lp_bytes, gateway_lp_data_address).await?; // UDP:51264
    /// ```
    pub fn wrap_data(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let state_machine = self
            .state_machine
            .as_mut()
            .ok_or_else(|| LpClientError::transport("Cannot wrap data: handshake not completed"))?;

        // Process data through state machine to create LP packet
        let action = state_machine
            .process_input(LpInput::SendData(data.to_vec()))
            .ok_or_else(|| LpClientError::transport("State machine returned no action"))?
            .map_err(|e| LpClientError::Transport(format!("Failed to encrypt data: {e}")))?;

        let packet = match action {
            LpAction::SendPacket(packet) => packet,
            other => {
                return Err(LpClientError::Transport(format!(
                    "Unexpected action when wrapping data: {:?}",
                    other
                )));
            }
        };

        // Get outer AEAD key for encryption
        let outer_key = state_machine
            .session()
            .ok()
            .and_then(|s| s.outer_aead_key_for_sending());

        // Serialize the packet with outer AEAD encryption
        let mut buf = BytesMut::new();
        serialize_lp_packet(&packet, &mut buf, outer_key.as_ref())
            .map_err(|e| LpClientError::Transport(format!("Failed to serialize LP packet: {e}")))?;

        Ok(buf.to_vec())
    }

    /// Get the LP session ID (receiver_idx) for this client.
    ///
    /// This ID is included in the outer header of LP packets and is used by
    /// the gateway to look up the session for decryption.
    ///
    /// # Returns
    /// * `Ok(u32)` - The session ID
    ///
    /// # Errors
    /// Returns an error if handshake has not been completed.
    pub fn session_id(&self) -> Result<u32> {
        let state_machine = self.state_machine.as_ref().ok_or_else(|| {
            LpClientError::transport("Cannot get session ID: handshake not completed")
        })?;

        state_machine
            .session()
            .map(|s| s.id())
            .map_err(|e| LpClientError::Transport(format!("Failed to get session: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let mut rng = rand::thread_rng();
        let keypair = Arc::new(ed25519::KeyPair::new(&mut rng));
        let gateway_key = *ed25519::KeyPair::new(&mut rng).public_key();
        let address = "127.0.0.1:41264".parse().unwrap();
        let client_ip = "192.168.1.100".parse().unwrap();

        let client =
            LpRegistrationClient::new_with_default_psk(keypair, gateway_key, address, client_ip);

        assert!(!client.is_handshake_complete());
        assert_eq!(client.gateway_address(), address);
        assert_eq!(client.client_ip(), client_ip);
    }
}
