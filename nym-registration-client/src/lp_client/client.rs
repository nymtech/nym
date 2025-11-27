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
use nym_lp::codec::{parse_lp_packet, serialize_lp_packet, OuterAeadKey};
use nym_lp::message::ForwardPacketData;
use nym_lp::state_machine::{LpAction, LpInput, LpStateMachine};
use nym_registration_common::{GatewayData, LpRegistrationRequest, LpRegistrationResponse};
use nym_wireguard_types::PeerPublicKey;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// LP (Lewes Protocol) registration client for direct gateway connections.
///
/// This client uses a packet-per-connection model where each LP packet
/// exchange opens a new TCP connection, sends one packet, receives one
/// response, then closes. Session state is maintained in the state machine
/// across connections.
///
/// # Example Flow
/// ```ignore
/// let mut client = LpRegistrationClient::new(...);
/// client.perform_handshake().await?;            // Noise handshake (multiple connections)
/// let gateway_data = client.register(...).await?;  // Registration (single connection)
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

    /// Performs the LP Noise protocol handshake with the gateway.
    ///
    /// This establishes a secure encrypted session using the Noise protocol.
    /// Uses packet-per-connection model: each handshake message opens a new
    /// TCP connection.
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
    /// 1. Sends ClientHello, receives Ack (connection 1)
    /// 2. Creates LP state machine with client as initiator
    /// 3. Exchanges handshake messages (each on new connection)
    /// 4. Stores the established session in the state machine
    pub async fn perform_handshake(&mut self) -> Result<()> {
        // Apply handshake timeout (nym-102)
        tokio::time::timeout(
            self.config.handshake_timeout,
            self.perform_handshake_inner(),
        )
        .await
        .map_err(|_| {
            LpClientError::Transport(format!(
                "Handshake timeout after {:?}",
                self.config.handshake_timeout
            ))
        })?
    }

    /// Internal handshake implementation without timeout.
    ///
    /// Uses packet-per-connection model: each LP packet exchange opens a new
    /// TCP connection, sends one packet, receives one response, then closes.
    async fn perform_handshake_inner(&mut self) -> Result<()> {
        tracing::debug!("Starting LP handshake as initiator (packet-per-connection)");

        // Step 1: Derive X25519 keys from Ed25519 for Noise protocol (internal to ClientHello)
        // The Ed25519 keys are used for PSQ authentication and also converted to X25519
        let client_x25519_public = self
            .local_ed25519_keypair
            .public_key()
            .to_x25519()
            .map_err(|e| {
                LpClientError::Crypto(format!("Failed to derive X25519 public key: {}", e))
            })?;

        // Step 2: Generate ClientHelloData with fresh salt and both public keys
        let client_hello_data = nym_lp::ClientHelloData::new_with_fresh_salt(
            client_x25519_public.to_bytes(),
            self.local_ed25519_keypair.public_key().to_bytes(),
        );
        let salt = client_hello_data.salt;
        let receiver_index = client_hello_data.receiver_index;

        tracing::trace!(
            "Generated ClientHello with timestamp: {}, receiver_index: {}",
            client_hello_data.extract_timestamp(),
            receiver_index
        );

        // Step 3: Send ClientHello and receive Ack (packet-per-connection)
        let client_hello_header = nym_lp::packet::LpHeader::new(
            nym_lp::BOOTSTRAP_RECEIVER_IDX, // session_id not yet established
            0,                              // counter starts at 0
        );
        let client_hello_packet = nym_lp::LpPacket::new(
            client_hello_header,
            nym_lp::LpMessage::ClientHello(client_hello_data),
        );

        let ack_response = Self::connect_send_receive(
            self.gateway_lp_address,
            &client_hello_packet,
            None, // No outer key before handshake
            &self.config,
        )
        .await?;

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

        // Step 6: Handshake loop - each packet on new connection
        loop {
            // Send pending packet if we have one
            if let Some(packet) = pending_packet.take() {
                // Get outer key from session (None before PSK, Some after)
                let outer_key = state_machine
                    .session()
                    .ok()
                    .and_then(|s| s.outer_aead_key());

                tracing::trace!("Sending handshake packet (outer_key={})", outer_key.is_some());
                let response = Self::connect_send_receive(
                    self.gateway_lp_address,
                    &packet,
                    outer_key.as_ref(),
                    &self.config,
                )
                .await?;
                tracing::trace!("Received handshake response");

                // Process the received packet
                if let Some(action) =
                    state_machine.process_input(LpInput::ReceivePacket(response))
                {
                    match action? {
                        LpAction::SendPacket(response_packet) => {
                            // Queue the response packet to send on next iteration
                            pending_packet = Some(response_packet);

                            // Check if handshake completed after queueing this packet
                            if state_machine.session()?.is_handshake_complete() {
                                // Send the final packet before breaking
                                if let Some(final_packet) = pending_packet.take() {
                                    let outer_key = state_machine
                                        .session()
                                        .ok()
                                        .and_then(|s| s.outer_aead_key());
                                    tracing::trace!("Sending final handshake packet");
                                    let ack_response = Self::connect_send_receive(
                                        self.gateway_lp_address,
                                        &final_packet,
                                        outer_key.as_ref(),
                                        &self.config,
                                    )
                                    .await?;

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
                                    LpClientError::Transport(
                                        "No handshake message available after KKT".to_string(),
                                    )
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
                return Err(LpClientError::Transport(
                    "Handshake stalled: no packet to send".to_string(),
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
    async fn connect_send_receive(
        address: SocketAddr,
        packet: &LpPacket,
        outer_key: Option<&OuterAeadKey>,
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

        // 3. Send packet with optional outer AEAD
        Self::send_packet_with_key(&mut stream, packet, outer_key).await?;

        // 4. Receive response with optional outer AEAD
        let response = Self::receive_packet_with_key(&mut stream, outer_key).await?;

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
            .map_err(|e| LpClientError::Transport(format!("Failed to serialize packet: {}", e)))?;

        // Send 4-byte length prefix (u32 big-endian)
        let len = packet_buf.len() as u32;
        stream.write_all(&len.to_be_bytes()).await.map_err(|e| {
            LpClientError::Transport(format!("Failed to send packet length: {}", e))
        })?;

        // Send the actual packet data
        stream
            .write_all(&packet_buf)
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to send packet data: {}", e)))?;

        // Flush to ensure data is sent immediately
        stream
            .flush()
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to flush stream: {}", e)))?;

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
        stream.read_exact(&mut len_buf).await.map_err(|e| {
            LpClientError::Transport(format!("Failed to read packet length: {}", e))
        })?;

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
        stream
            .read_exact(&mut packet_buf)
            .await
            .map_err(|e| LpClientError::Transport(format!("Failed to read packet data: {}", e)))?;

        let packet = parse_lp_packet(&packet_buf, outer_key)
            .map_err(|e| LpClientError::Transport(format!("Failed to parse packet: {}", e)))?;

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
    /// Uses packet-per-connection model: opens connection, sends request,
    /// receives response, closes connection.
    ///
    /// # Arguments
    /// * `wg_keypair` - Client's WireGuard x25519 keypair
    /// * `credential` - Pre-generated bandwidth credential
    /// * `ticket_type` - Type of bandwidth ticket
    ///
    /// # Returns
    /// * `Ok(GatewayData)` - Gateway configuration data on successful registration
    pub async fn register_with_credential(
        &mut self,
        wg_keypair: &x25519::KeyPair,
        credential: CredentialSpendingData,
        ticket_type: TicketType,
    ) -> Result<GatewayData> {
        // Ensure handshake is complete (state machine exists)
        let state_machine = self.state_machine.as_mut().ok_or_else(|| {
            LpClientError::Transport(
                "Cannot register: handshake not completed".to_string(),
            )
        })?;

        tracing::debug!("Sending registration request (packet-per-connection)");

        // 1. Build registration request
        let wg_public_key = PeerPublicKey::new(wg_keypair.public_key().to_bytes().into());
        let request =
            LpRegistrationRequest::new_dvpn(wg_public_key, credential, ticket_type, self.client_ip);

        tracing::trace!("Built registration request: {:?}", request);

        // 2. Serialize the request
        let request_bytes = bincode::serialize(&request).map_err(|e| {
            LpClientError::SendRegistrationRequest(format!("Failed to serialize request: {}", e))
        })?;

        tracing::debug!(
            "Sending registration request ({} bytes)",
            request_bytes.len()
        );

        // 3. Encrypt and prepare packet via state machine
        let action = state_machine
            .process_input(LpInput::SendData(request_bytes))
            .ok_or_else(|| {
                LpClientError::Transport("State machine returned no action".to_string())
            })?
            .map_err(|e| {
                LpClientError::SendRegistrationRequest(format!(
                    "Failed to encrypt registration request: {}",
                    e
                ))
            })?;

        let request_packet = match action {
            LpAction::SendPacket(packet) => packet,
            other => {
                return Err(LpClientError::Transport(format!(
                    "Unexpected action when sending registration data: {:?}",
                    other
                )));
            }
        };

        // 4. Get outer key from session
        let outer_key = state_machine
            .session()
            .ok()
            .and_then(|s| s.outer_aead_key());

        // 5. Send request and receive response on fresh connection with timeout
        let response_packet = tokio::time::timeout(
            self.config.registration_timeout,
            Self::connect_send_receive(
                self.gateway_lp_address,
                &request_packet,
                outer_key.as_ref(),
                &self.config,
            ),
        )
        .await
        .map_err(|_| {
            LpClientError::ReceiveRegistrationResponse(format!(
                "Registration timeout after {:?}",
                self.config.registration_timeout
            ))
        })??;

        tracing::trace!("Received registration response packet");

        // 6. Decrypt via state machine
        let action = state_machine
            .process_input(LpInput::ReceivePacket(response_packet))
            .ok_or_else(|| {
                LpClientError::Transport("State machine returned no action".to_string())
            })?
            .map_err(|e| {
                LpClientError::ReceiveRegistrationResponse(format!(
                    "Failed to decrypt registration response: {}",
                    e
                ))
            })?;

        // 7. Extract decrypted data
        let response_data = match action {
            LpAction::DeliverData(data) => data,
            other => {
                return Err(LpClientError::Transport(format!(
                    "Unexpected action when receiving registration response: {:?}",
                    other
                )));
            }
        };

        // 8. Deserialize the response
        let response: LpRegistrationResponse =
            bincode::deserialize(&response_data).map_err(|e| {
                LpClientError::ReceiveRegistrationResponse(format!(
                    "Failed to deserialize registration response: {}",
                    e
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
            tracing::warn!("Gateway rejected registration: {}", error_msg);
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

    /// Sends a ForwardPacket message to the entry gateway for forwarding to the exit gateway.
    ///
    /// This method constructs a ForwardPacket containing the target gateway's identity,
    /// address, and the inner LP packet bytes, encrypts it through the outer session
    /// (client-entry), and receives the response from the exit gateway via the entry gateway.
    ///
    /// Uses packet-per-connection model: opens connection, sends forward packet,
    /// receives response, closes connection.
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
        // Ensure handshake is complete (state machine exists)
        let state_machine = self.state_machine.as_mut().ok_or_else(|| {
            LpClientError::Transport(
                "Cannot send forward packet: handshake not completed".to_string(),
            )
        })?;

        tracing::debug!(
            "Sending ForwardPacket to {} ({} inner bytes, packet-per-connection)",
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
            LpClientError::Transport(format!("Failed to serialize ForwardPacketData: {}", e))
        })?;

        tracing::trace!(
            "Serialized ForwardPacketData ({} bytes)",
            forward_data_bytes.len()
        );

        // 3. Encrypt and prepare packet via state machine
        let action = state_machine
            .process_input(LpInput::SendData(forward_data_bytes))
            .ok_or_else(|| {
                LpClientError::Transport("State machine returned no action".to_string())
            })?
            .map_err(|e| {
                LpClientError::Transport(format!("Failed to encrypt ForwardPacket: {}", e))
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

        // 4. Get outer key from session
        let outer_key = state_machine
            .session()
            .ok()
            .and_then(|s| s.outer_aead_key());

        // 5. Send and receive on fresh connection
        let response_packet = Self::connect_send_receive(
            self.gateway_lp_address,
            &forward_packet,
            outer_key.as_ref(),
            &self.config,
        )
        .await?;
        tracing::trace!("Received response packet from entry gateway");

        // 6. Decrypt via state machine
        let action = state_machine
            .process_input(LpInput::ReceivePacket(response_packet))
            .ok_or_else(|| {
                LpClientError::Transport("State machine returned no action".to_string())
            })?
            .map_err(|e| {
                LpClientError::Transport(format!("Failed to decrypt forward response: {}", e))
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
