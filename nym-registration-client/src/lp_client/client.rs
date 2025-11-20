// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) registration client for direct gateway connections.

use super::config::LpConfig;
use super::error::{LpClientError, Result};
use super::transport::LpTransport;
use bytes::BytesMut;
use nym_bandwidth_controller::{BandwidthTicketProvider, DEFAULT_TICKETS_TO_SPEND};
use nym_credentials_interface::{CredentialSpendingData, TicketType};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::LpPacket;
use nym_lp::codec::{parse_lp_packet, serialize_lp_packet};
use nym_lp::state_machine::{LpAction, LpInput, LpStateMachine};
use nym_registration_common::{GatewayData, LpRegistrationRequest, LpRegistrationResponse};
use nym_wireguard_types::PeerPublicKey;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// LP (Lewes Protocol) registration client for direct gateway connections.
///
/// This client manages:
/// - TCP connection to the gateway's LP listener
/// - Noise protocol handshake via LP state machine
/// - Registration request/response exchange
/// - Encrypted transport after handshake
///
/// # Example Flow
/// ```ignore
/// let client = LpRegistrationClient::new(...);
/// client.connect().await?;                 // nym-78: Establish TCP
/// client.perform_handshake().await?;       // nym-79: Noise handshake
/// let response = client.register(...).await?;  // nym-80: Send registration
/// ```
pub struct LpRegistrationClient {
    /// TCP stream connection to the gateway.
    /// Created during `connect()`, None before connection is established.
    tcp_stream: Option<TcpStream>,

    /// Client's Ed25519 identity keypair (used for PSQ authentication and X25519 derivation).
    local_ed25519_keypair: Arc<ed25519::KeyPair>,

    /// Gateway's Ed25519 public key (from directory/discovery).
    gateway_ed25519_public_key: ed25519::PublicKey,

    /// Gateway LP listener address (host:port, e.g., "1.1.1.1:41264").
    gateway_lp_address: SocketAddr,

    /// LP state machine for managing connection lifecycle.
    /// Created during handshake initiation (nym-79).
    state_machine: Option<LpStateMachine>,

    /// Client's IP address for registration metadata.
    client_ip: IpAddr,

    /// Configuration for timeouts and TCP parameters (nym-87, nym-102, nym-104).
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
    /// This creates the client but does not establish the connection.
    /// Call `connect()` to establish the TCP connection.
    /// PSK is derived automatically during handshake inside the state machine.
    pub fn new(
        local_ed25519_keypair: Arc<ed25519::KeyPair>,
        gateway_ed25519_public_key: ed25519::PublicKey,
        gateway_lp_address: SocketAddr,
        client_ip: IpAddr,
        config: LpConfig,
    ) -> Self {
        Self {
            tcp_stream: None,
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

    /// Establishes TCP connection to the gateway's LP listener.
    ///
    /// This must be called before attempting handshake or registration.
    ///
    /// # Errors
    /// Returns `LpClientError::TcpConnection` if the connection fails or times out.
    ///
    /// # Implementation Note
    /// This is implemented in nym-78. The handshake (nym-79) and registration
    /// (nym-80, nym-81) will be added in subsequent tasks.
    /// Timeout and TCP parameters added in nym-102 and nym-104.
    pub async fn connect(&mut self) -> Result<()> {
        // Apply connect timeout (nym-102)
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

        // Apply TCP_NODELAY (nym-104)
        stream
            .set_nodelay(self.config.tcp_nodelay)
            .map_err(|source| LpClientError::TcpConnection {
                address: self.gateway_lp_address.to_string(),
                source,
            })?;

        tracing::info!(
            "Successfully connected to gateway LP listener at {} (timeout={:?}, nodelay={})",
            self.gateway_lp_address,
            self.config.connect_timeout,
            self.config.tcp_nodelay
        );

        self.tcp_stream = Some(stream);
        Ok(())
    }

    /// Returns a reference to the TCP stream if connected.
    pub fn tcp_stream(&self) -> Option<&TcpStream> {
        self.tcp_stream.as_ref()
    }

    /// Returns whether the client is currently connected via TCP.
    pub fn is_connected(&self) -> bool {
        self.tcp_stream.is_some()
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
    /// Must be called after `connect()` and before attempting registration.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Not connected via TCP
    /// - State machine creation fails
    /// - Handshake protocol fails
    /// - Network communication fails
    /// - Handshake times out (see LpConfig::handshake_timeout)
    ///
    /// # Implementation
    /// This implements the Noise protocol handshake as the initiator:
    /// 1. Creates LP state machine with client as initiator
    /// 2. Sends initial handshake packet
    /// 3. Exchanges handshake messages until complete
    /// 4. Stores the established session in the state machine
    ///
    /// Timeout applied in nym-102.
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
    async fn perform_handshake_inner(&mut self) -> Result<()> {
        let stream = self.tcp_stream.as_mut().ok_or_else(|| {
            LpClientError::Transport("Cannot perform handshake: not connected".to_string())
        })?;

        tracing::debug!("Starting LP handshake as initiator");

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

        tracing::trace!(
            "Generated ClientHello with timestamp: {}",
            client_hello_data.extract_timestamp()
        );

        // Step 3: Send ClientHello as first packet (before Noise handshake)
        let client_hello_header = nym_lp::packet::LpHeader::new(
            0, // session_id not yet established
            0, // counter starts at 0
        );
        let client_hello_packet = nym_lp::LpPacket::new(
            client_hello_header,
            nym_lp::LpMessage::ClientHello(client_hello_data),
        );
        Self::send_packet(stream, &client_hello_packet).await?;
        tracing::debug!("Sent ClientHello packet");

        // Step 4: Create state machine as initiator with Ed25519 keys
        // PSK derivation happens internally in the state machine constructor
        let mut state_machine = LpStateMachine::new(
            true, // is_initiator
            (
                self.local_ed25519_keypair.private_key(),
                self.local_ed25519_keypair.public_key(),
            ),
            &self.gateway_ed25519_public_key,
            &salt,
        )?;

        // Start handshake - client (initiator) sends first
        if let Some(action) = state_machine.process_input(LpInput::StartHandshake) {
            match action? {
                LpAction::SendPacket(packet) => {
                    tracing::trace!("Sending initial handshake packet");
                    Self::send_packet(stream, &packet).await?;
                }
                other => {
                    return Err(LpClientError::Transport(format!(
                        "Unexpected action at handshake start: {:?}",
                        other
                    )));
                }
            }
        }

        // Continue handshake until complete
        loop {
            // Read incoming packet from gateway
            let packet = Self::receive_packet(stream).await?;
            tracing::trace!("Received handshake packet");

            // Process the received packet
            if let Some(action) = state_machine.process_input(LpInput::ReceivePacket(packet)) {
                match action? {
                    LpAction::SendPacket(response_packet) => {
                        tracing::trace!("Sending handshake response packet");
                        Self::send_packet(stream, &response_packet).await?;

                        // Check if handshake completed after sending this packet
                        // (e.g., initiator completes after sending final message)
                        if state_machine.session()?.is_handshake_complete() {
                            tracing::info!("LP handshake completed after sending packet");
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
                        let noise_msg = state_machine.session()?.prepare_handshake_message()
                            .ok_or_else(|| LpClientError::Transport("No handshake message available after KKT".to_string()))??;
                        let noise_packet = state_machine.session()?.next_packet(noise_msg)?;
                        tracing::trace!("Sending first Noise handshake message");
                        Self::send_packet(stream, &noise_packet).await?;
                    }
                    other => {
                        tracing::trace!("Received action during handshake: {:?}", other);
                    }
                }
            }
        }

        // Store the state machine (with established session) for later use
        self.state_machine = Some(state_machine);
        Ok(())
    }

    /// Sends an LP packet over the TCP stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Errors
    /// Returns an error if serialization or network transmission fails.
    async fn send_packet(stream: &mut TcpStream, packet: &LpPacket) -> Result<()> {
        // Serialize the packet
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(packet, &mut packet_buf)
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

    /// Receives an LP packet from the TCP stream with length-prefixed framing.
    ///
    /// Format: 4-byte big-endian u32 length + packet bytes
    ///
    /// # Errors
    /// Returns an error if:
    /// - Network read fails
    /// - Packet size exceeds maximum (64KB)
    /// - Packet parsing fails
    async fn receive_packet(stream: &mut TcpStream) -> Result<LpPacket> {
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

        // Parse the packet
        let packet = parse_lp_packet(&packet_buf)
            .map_err(|e| LpClientError::Transport(format!("Failed to parse packet: {}", e)))?;

        tracing::trace!("Received LP packet ({} bytes + 4 byte header)", packet_len);
        Ok(packet)
    }

    /// Sends an encrypted registration request to the gateway.
    ///
    /// This must be called after a successful handshake. The registration request
    /// includes the client's WireGuard public key, bandwidth credential, and other
    /// registration metadata.
    ///
    /// # Arguments
    /// * `wg_keypair` - Client's WireGuard x25519 keypair
    /// * `gateway_identity` - Gateway's ed25519 identity for credential verification
    /// * `bandwidth_controller` - Provider for bandwidth credentials
    /// * `ticket_type` - Type of bandwidth ticket to use
    ///
    /// # Errors
    /// Returns an error if:
    /// - No connection is established
    /// - Handshake has not been completed
    /// - Credential acquisition fails
    /// - Request serialization fails
    /// - Encryption or network transmission fails
    ///
    /// # Implementation Note (nym-80)
    /// This implements the LP registration request sending:
    /// 1. Acquires bandwidth credential from controller
    /// 2. Constructs LpRegistrationRequest with dVPN mode
    /// 3. Serializes request to bytes using bincode
    /// 4. Encrypts via LP state machine (LpInput::SendData)
    /// 5. Sends encrypted packet to gateway
    pub async fn send_registration_request(
        &mut self,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
    ) -> Result<()> {
        // Ensure we have a TCP connection
        let stream = self.tcp_stream.as_mut().ok_or_else(|| {
            LpClientError::Transport("Cannot send registration: not connected".to_string())
        })?;

        // Ensure handshake is complete (state machine exists and is in Transport state)
        let state_machine = self.state_machine.as_mut().ok_or_else(|| {
            LpClientError::Transport(
                "Cannot send registration: handshake not completed".to_string(),
            )
        })?;

        tracing::debug!("Acquiring bandwidth credential for registration");

        // 1. Get bandwidth credential from controller
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

        // 2. Build registration request
        let wg_public_key = PeerPublicKey::new(wg_keypair.public_key().to_bytes().into());
        let request =
            LpRegistrationRequest::new_dvpn(wg_public_key, credential, ticket_type, self.client_ip);

        tracing::trace!("Built registration request: {:?}", request);

        // 3. Serialize the request
        let request_bytes = bincode::serialize(&request).map_err(|e| {
            LpClientError::SendRegistrationRequest(format!("Failed to serialize request: {}", e))
        })?;

        tracing::debug!(
            "Sending registration request ({} bytes)",
            request_bytes.len()
        );

        // 4. Encrypt and prepare packet via state machine
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

        // 5. Send the encrypted packet
        match action {
            LpAction::SendPacket(packet) => {
                Self::send_packet(stream, &packet).await?;
                tracing::info!("Successfully sent registration request to gateway");
                Ok(())
            }
            other => Err(LpClientError::Transport(format!(
                "Unexpected action when sending registration data: {:?}",
                other
            ))),
        }
    }

    /// Sends LP registration request with a pre-generated credential.
    /// This is useful for testing with mock ecash credentials.
    ///
    /// This implements the LP registration request sending:
    /// 1. Uses pre-provided bandwidth credential (skips acquisition)
    /// 2. Constructs LpRegistrationRequest with dVPN mode
    /// 3. Serializes request to bytes using bincode
    /// 4. Encrypts via LP state machine (LpInput::SendData)
    /// 5. Sends encrypted packet to gateway
    pub async fn send_registration_request_with_credential(
        &mut self,
        wg_keypair: &x25519::KeyPair,
        _gateway_identity: &ed25519::PublicKey,
        credential: CredentialSpendingData,
        ticket_type: TicketType,
    ) -> Result<()> {
        // Ensure we have a TCP connection
        let stream = self.tcp_stream.as_mut().ok_or_else(|| {
            LpClientError::Transport("Cannot send registration: not connected".to_string())
        })?;

        // Ensure handshake is complete (state machine exists and is in Transport state)
        let state_machine = self.state_machine.as_mut().ok_or_else(|| {
            LpClientError::Transport(
                "Cannot send registration: handshake not completed".to_string(),
            )
        })?;

        tracing::debug!("Using pre-generated credential for registration");

        // Build registration request with pre-generated credential
        let wg_public_key = PeerPublicKey::new(wg_keypair.public_key().to_bytes().into());
        let request =
            LpRegistrationRequest::new_dvpn(wg_public_key, credential, ticket_type, self.client_ip);

        tracing::trace!("Built registration request: {:?}", request);

        // Serialize the request
        let request_bytes = bincode::serialize(&request).map_err(|e| {
            LpClientError::SendRegistrationRequest(format!("Failed to serialize request: {}", e))
        })?;

        tracing::debug!(
            "Sending registration request ({} bytes)",
            request_bytes.len()
        );

        // Encrypt and prepare packet via state machine
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

        // Send the encrypted packet
        match action {
            LpAction::SendPacket(packet) => {
                Self::send_packet(stream, &packet).await?;
                tracing::info!("Successfully sent registration request to gateway");
                Ok(())
            }
            other => Err(LpClientError::Transport(format!(
                "Unexpected action when sending registration data: {:?}",
                other
            ))),
        }
    }

    /// Receives and processes the registration response from the gateway.
    ///
    /// This must be called after sending a registration request. The method:
    /// 1. Receives an encrypted response packet from the gateway
    /// 2. Decrypts it using the established LP session
    /// 3. Deserializes the LpRegistrationResponse
    /// 4. Validates the response and extracts GatewayData
    ///
    /// # Returns
    /// * `Ok(GatewayData)` - Gateway configuration data on successful registration
    ///
    /// # Errors
    /// Returns an error if:
    /// - No connection is established
    /// - Handshake has not been completed
    /// - Network reception fails
    /// - Decryption fails
    /// - Response deserialization fails
    /// - Gateway rejected the registration (success=false)
    /// - Response is missing gateway_data
    /// - Response times out (see LpConfig::registration_timeout)
    ///
    /// # Implementation Note (nym-81)
    /// This implements the LP registration response processing:
    /// 1. Receives length-prefixed packet from TCP stream
    /// 2. Processes via state machine (LpInput::ReceivePacket)
    /// 3. Extracts decrypted data from LpAction::DeliverData
    /// 4. Deserializes as LpRegistrationResponse
    /// 5. Validates and returns GatewayData
    ///
    /// Timeout applied in nym-102.
    pub async fn receive_registration_response(&mut self) -> Result<GatewayData> {
        // Apply registration timeout (nym-102)
        tokio::time::timeout(
            self.config.registration_timeout,
            self.receive_registration_response_inner(),
        )
        .await
        .map_err(|_| {
            LpClientError::ReceiveRegistrationResponse(format!(
                "Registration response timeout after {:?}",
                self.config.registration_timeout
            ))
        })?
    }

    /// Internal registration response implementation without timeout.
    async fn receive_registration_response_inner(&mut self) -> Result<GatewayData> {
        // Ensure we have a TCP connection
        let stream = self.tcp_stream.as_mut().ok_or_else(|| {
            LpClientError::Transport(
                "Cannot receive registration response: not connected".to_string(),
            )
        })?;

        // Ensure handshake is complete (state machine exists)
        let state_machine = self.state_machine.as_mut().ok_or_else(|| {
            LpClientError::Transport(
                "Cannot receive registration response: handshake not completed".to_string(),
            )
        })?;

        tracing::debug!("Waiting for registration response from gateway");

        // 1. Receive the response packet
        let packet = Self::receive_packet(stream).await?;

        tracing::trace!("Received registration response packet");

        // 2. Decrypt via state machine
        let action = state_machine
            .process_input(LpInput::ReceivePacket(packet))
            .ok_or_else(|| {
                LpClientError::Transport("State machine returned no action".to_string())
            })?
            .map_err(|e| {
                LpClientError::ReceiveRegistrationResponse(format!(
                    "Failed to decrypt registration response: {}",
                    e
                ))
            })?;

        // 3. Extract decrypted data
        let response_data = match action {
            LpAction::DeliverData(data) => data,
            other => {
                return Err(LpClientError::Transport(format!(
                    "Unexpected action when receiving registration response: {:?}",
                    other
                )));
            }
        };

        // 4. Deserialize the response
        let response: LpRegistrationResponse =
            bincode::deserialize(&response_data).map_err(|e| {
                LpClientError::ReceiveRegistrationResponse(format!(
                    "Failed to deserialize registration response: {}",
                    e
                ))
            })?;

        tracing::debug!(
            "Received registration response: success={}, session_id={}",
            response.success,
            response.session_id
        );

        // 5. Validate and extract GatewayData
        if !response.success {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            tracing::warn!("Gateway rejected registration: {}", error_msg);
            return Err(LpClientError::RegistrationRejected { reason: error_msg });
        }

        // Extract gateway_data
        let gateway_data = response.gateway_data.ok_or_else(|| {
            LpClientError::ReceiveRegistrationResponse(
                "Gateway response missing gateway_data despite success=true".to_string(),
            )
        })?;

        tracing::info!(
            "LP registration successful! Session ID: {}, Allocated bandwidth: {} bytes",
            response.session_id,
            response.allocated_bandwidth
        );

        Ok(gateway_data)
    }

    /// Converts this client into an LpTransport for ongoing post-handshake communication.
    ///
    /// This consumes the client and transfers ownership of the TCP stream and state machine
    /// to a new LpTransport instance, which can be used for arbitrary data transfer.
    ///
    /// # Returns
    /// * `Ok(LpTransport)` - Transport handler for ongoing communication
    ///
    /// # Errors
    /// Returns an error if:
    /// - No connection is established
    /// - Handshake has not been completed
    /// - State machine is not in Transport state
    ///
    /// # Example
    /// ```ignore
    /// let mut client = LpRegistrationClient::new(...);
    /// client.connect().await?;
    /// client.perform_handshake().await?;
    /// // After registration is complete...
    /// let mut transport = client.into_transport()?;
    /// transport.send_data(b"hello").await?;
    /// ```
    ///
    /// # Implementation Note (nym-82)
    /// This enables ongoing communication after registration by transferring
    /// the established LP session to a dedicated transport handler.
    pub fn into_transport(self) -> Result<LpTransport> {
        // Ensure connection exists
        let stream = self.tcp_stream.ok_or_else(|| {
            LpClientError::Transport(
                "Cannot create transport: no TCP connection established".to_string(),
            )
        })?;

        // Ensure handshake completed
        let state_machine = self.state_machine.ok_or_else(|| {
            LpClientError::Transport("Cannot create transport: handshake not completed".to_string())
        })?;

        // Create and return transport (validates state is Transport)
        LpTransport::from_handshake(stream, state_machine)
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

        assert!(!client.is_connected());
        assert_eq!(client.gateway_address(), address);
        assert_eq!(client.client_ip(), client_ip);
    }
}
