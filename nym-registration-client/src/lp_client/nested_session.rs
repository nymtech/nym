// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Nested LP session for client-exit handshake through entry gateway forwarding.
//!
//! This module implements the inner LP session management where a client establishes
//! a secure connection with an exit gateway by forwarding LP packets through an
//! entry gateway. This hides the client's IP address from the exit gateway.
//!
//! # Architecture
//!
//! ```text
//! Client ←→ Entry Gateway (outer session, encrypted)
//!              ↓ forwards
//!           Exit Gateway (inner session, client establishes handshake)
//! ```
//!
//! The entry gateway sees the client's IP but doesn't know the final destination.
//! The exit gateway processes the LP handshake but only sees the entry gateway's IP.

use super::client::LpRegistrationClient;
use super::error::{LpClientError, Result};
use bytes::BytesMut;
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::codec::{parse_lp_packet, serialize_lp_packet, OuterAeadKey};
use nym_lp::state_machine::{LpAction, LpInput, LpStateMachine};
use nym_lp::{LpMessage, LpPacket};
use nym_registration_common::{GatewayData, LpRegistrationRequest, LpRegistrationResponse};
use nym_wireguard_types::PeerPublicKey;
use std::net::IpAddr;
use std::sync::Arc;

/// Manages a nested LP session where the client establishes a handshake with
/// an exit gateway by forwarding packets through an entry gateway.
///
/// # Example
///
/// ```ignore
/// // Outer session already established with entry gateway
/// let mut outer_client = LpRegistrationClient::new(...);
/// outer_client.perform_handshake().await?;
///
/// // Now establish inner session with exit gateway
/// let mut nested = NestedLpSession::new(
///     exit_identity,
///     "2.2.2.2:41264".to_string(),
///     client_keypair,
///     exit_public_key,
/// );
///
/// let gateway_data = nested.handshake_and_register(&mut outer_client, ...).await?;
/// ```
pub struct NestedLpSession {
    /// Exit gateway's Ed25519 identity (32 bytes)
    exit_identity: [u8; 32],

    /// Exit gateway's LP address (e.g., "2.2.2.2:41264")
    exit_address: String,

    /// Client's Ed25519 keypair (for PSQ authentication and X25519 derivation)
    client_keypair: Arc<ed25519::KeyPair>,

    /// Exit gateway's Ed25519 public key
    exit_public_key: ed25519::PublicKey,

    /// LP state machine for exit gateway session (populated after handshake)
    state_machine: Option<LpStateMachine>,
}

impl NestedLpSession {
    /// Creates a new nested LP session handler.
    ///
    /// # Arguments
    /// * `exit_identity` - Exit gateway's Ed25519 identity (32 bytes)
    /// * `exit_address` - Exit gateway's LP address (e.g., "2.2.2.2:41264")
    /// * `client_keypair` - Client's Ed25519 keypair
    /// * `exit_public_key` - Exit gateway's Ed25519 public key
    pub fn new(
        exit_identity: [u8; 32],
        exit_address: String,
        client_keypair: Arc<ed25519::KeyPair>,
        exit_public_key: ed25519::PublicKey,
    ) -> Self {
        Self {
            exit_identity,
            exit_address,
            client_keypair,
            exit_public_key,
            state_machine: None,
        }
    }

    /// Performs the LP handshake with the exit gateway by forwarding packets
    /// through the entry gateway.
    ///
    /// This method:
    /// 1. Generates ClientHello for exit gateway
    /// 2. Creates LP state machine for exit handshake
    /// 3. Runs handshake loop, forwarding all packets through entry gateway
    /// 4. Stores established session in internal state machine
    ///
    /// # Arguments
    /// * `outer_client` - Connected LP client with established outer session to entry gateway
    ///
    /// # Errors
    /// Returns an error if:
    /// - Packet serialization/parsing fails
    /// - Forwarding through entry gateway fails
    /// - Exit gateway handshake fails
    /// - Cryptographic operations fail
    async fn perform_handshake(
        &mut self,
        outer_client: &mut LpRegistrationClient,
    ) -> Result<()> {
        tracing::debug!(
            "Starting nested LP handshake with exit gateway {}",
            self.exit_address
        );

        // Step 1: Derive X25519 keys from Ed25519 for Noise protocol
        let client_x25519_public = self
            .client_keypair
            .public_key()
            .to_x25519()
            .map_err(|e| {
                LpClientError::Crypto(format!("Failed to derive X25519 public key: {}", e))
            })?;

        // Step 2: Generate ClientHello for exit gateway
        let client_hello_data = nym_lp::ClientHelloData::new_with_fresh_salt(
            client_x25519_public.to_bytes(),
            self.client_keypair.public_key().to_bytes(),
        );
        let salt = client_hello_data.salt;
        let receiver_index = client_hello_data.receiver_index;

        tracing::trace!(
            "Generated ClientHello for exit gateway (timestamp: {})",
            client_hello_data.extract_timestamp()
        );

        // Step 3: Send ClientHello to exit gateway via forwarding
        let client_hello_header = nym_lp::packet::LpHeader::new(
            nym_lp::BOOTSTRAP_RECEIVER_IDX, // Use constant for bootstrap session
            0,                               // counter starts at 0
        );
        let client_hello_packet = nym_lp::LpPacket::new(
            client_hello_header,
            LpMessage::ClientHello(client_hello_data),
        );

        // Serialize and forward ClientHello (no state machine yet, no outer key)
        let client_hello_bytes = Self::serialize_packet(&client_hello_packet, None)?;
        let _response_bytes = outer_client
            .send_forward_packet(
                self.exit_identity,
                self.exit_address.clone(),
                client_hello_bytes,
            )
            .await?;

        tracing::debug!("Sent ClientHello to exit gateway via entry");

        // Step 4: Create state machine for exit gateway handshake
        let mut state_machine = LpStateMachine::new(
            receiver_index,
            true, // is_initiator
            (
                self.client_keypair.private_key(),
                self.client_keypair.public_key(),
            ),
            &self.exit_public_key,
            &salt,
        )?;

        // Step 5: Get initial packet from StartHandshake
        let mut pending_packet: Option<LpPacket> = None;
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

        // Step 6: Handshake loop - each packet on new connection via forwarding
        loop {
            if let Some(packet) = pending_packet.take() {
                tracing::trace!("Sending handshake packet to exit via forwarding");
                let response = self
                    .send_and_receive_via_forward(outer_client, &state_machine, &packet)
                    .await?;
                tracing::trace!("Received handshake response from exit");

                // Process the received packet
                if let Some(action) =
                    state_machine.process_input(LpInput::ReceivePacket(response))
                {
                    match action? {
                        LpAction::SendPacket(response_packet) => {
                            pending_packet = Some(response_packet);

                            // Check if handshake completed - send final packet if so
                            if state_machine.session()?.is_handshake_complete() {
                                if let Some(final_packet) = pending_packet.take() {
                                    tracing::trace!("Sending final handshake packet to exit");
                                    let _ = self
                                        .send_and_receive_via_forward(
                                            outer_client,
                                            &state_machine,
                                            &final_packet,
                                        )
                                        .await?;
                                }
                                tracing::info!(
                                    "Nested LP handshake completed with exit gateway"
                                );
                                break;
                            }
                        }
                        LpAction::HandshakeComplete => {
                            tracing::info!("Nested LP handshake completed with exit gateway");
                            break;
                        }
                        LpAction::KKTComplete => {
                            tracing::info!("KKT exchange completed with exit, starting Noise");
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
                    "Nested handshake stalled: no packet to send".to_string(),
                ));
            }
        }

        // Store the state machine (with established session) for later use
        self.state_machine = Some(state_machine);
        Ok(())
    }

    /// Performs handshake and registration with the exit gateway via forwarding.
    ///
    /// This is the main entry point for nested LP registration. It:
    /// 1. Performs handshake with exit gateway (via `perform_handshake`)
    /// 2. Builds and sends registration request through the forwarded connection
    /// 3. Receives and processes registration response
    /// 4. Returns gateway data on successful registration
    ///
    /// # Arguments
    /// * `outer_client` - Connected LP client with established outer session to entry gateway
    /// * `wg_keypair` - Client's WireGuard x25519 keypair
    /// * `gateway_identity` - Exit gateway's Ed25519 identity (for credential verification)
    /// * `bandwidth_controller` - Provider for bandwidth credentials
    /// * `ticket_type` - Type of bandwidth ticket to use
    /// * `client_ip` - Client IP address for registration metadata
    ///
    /// # Returns
    /// * `Ok(GatewayData)` - Exit gateway configuration data on successful registration
    ///
    /// # Errors
    /// Returns an error if:
    /// - Handshake fails
    /// - Credential acquisition fails
    /// - Request serialization/encryption fails
    /// - Forwarding through entry gateway fails
    /// - Response decryption/deserialization fails
    /// - Gateway rejects the registration
    pub async fn handshake_and_register(
        &mut self,
        outer_client: &mut LpRegistrationClient,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
        client_ip: IpAddr,
    ) -> Result<GatewayData> {
        // Step 1: Perform handshake with exit gateway via forwarding
        self.perform_handshake(outer_client).await?;

        // Step 2: Get the state machine (must exist after successful handshake)
        let state_machine = self.state_machine.as_mut().ok_or_else(|| {
            LpClientError::Transport("State machine missing after handshake".to_string())
        })?;

        tracing::debug!("Building registration request for exit gateway");

        // Step 3: Acquire bandwidth credential
        let credential = bandwidth_controller
            .get_ecash_ticket(ticket_type, *gateway_identity, nym_bandwidth_controller::DEFAULT_TICKETS_TO_SPEND)
            .await
            .map_err(|e| {
                LpClientError::Transport(format!(
                    "Failed to acquire bandwidth credential: {}",
                    e
                ))
            })?
            .data;

        // Step 4: Build registration request
        let wg_public_key = PeerPublicKey::new(wg_keypair.public_key().to_bytes().into());
        let request = LpRegistrationRequest::new_dvpn(wg_public_key, credential, ticket_type, client_ip);

        tracing::trace!("Built registration request: {:?}", request);

        // Step 5: Serialize the request
        let request_bytes = bincode::serialize(&request).map_err(|e| {
            LpClientError::Transport(format!("Failed to serialize registration request: {}", e))
        })?;

        tracing::debug!(
            "Sending registration request to exit gateway via forwarding ({} bytes)",
            request_bytes.len()
        );

        // Step 6: Encrypt and prepare packet via state machine
        let action = state_machine
            .process_input(LpInput::SendData(request_bytes))
            .ok_or_else(|| {
                LpClientError::Transport("State machine returned no action".to_string())
            })?
            .map_err(|e| {
                LpClientError::Transport(format!(
                    "Failed to encrypt registration request: {}",
                    e
                ))
            })?;

        // Step 7: Send the encrypted packet via forwarding
        // Get outer key for AEAD encryption (PSK is available after handshake)
        let outer_key = state_machine.session().ok().and_then(|s| s.outer_aead_key());
        let response_bytes = match action {
            LpAction::SendPacket(packet) => {
                let packet_bytes = Self::serialize_packet(&packet, outer_key.as_ref())?;
                outer_client
                    .send_forward_packet(
                        self.exit_identity,
                        self.exit_address.clone(),
                        packet_bytes,
                    )
                    .await?
            }
            other => {
                return Err(LpClientError::Transport(format!(
                    "Unexpected action when sending registration data: {:?}",
                    other
                )));
            }
        };

        tracing::trace!("Received registration response from exit gateway");

        // Step 8: Parse response bytes to LP packet
        let outer_key = state_machine.session().ok().and_then(|s| s.outer_aead_key());
        let response_packet = Self::parse_packet(&response_bytes, outer_key.as_ref())?;

        // Step 9: Decrypt via state machine
        let action = state_machine
            .process_input(LpInput::ReceivePacket(response_packet))
            .ok_or_else(|| {
                LpClientError::Transport("State machine returned no action".to_string())
            })?
            .map_err(|e| {
                LpClientError::Transport(format!(
                    "Failed to decrypt registration response: {}",
                    e
                ))
            })?;

        // Step 10: Extract decrypted data
        let response_data = match action {
            LpAction::DeliverData(data) => data,
            other => {
                return Err(LpClientError::Transport(format!(
                    "Unexpected action when receiving registration response: {:?}",
                    other
                )));
            }
        };

        // Step 11: Deserialize the response
        let response: LpRegistrationResponse =
            bincode::deserialize(&response_data).map_err(|e| {
                LpClientError::Transport(format!(
                    "Failed to deserialize registration response: {}",
                    e
                ))
            })?;

        tracing::debug!(
            "Received registration response from exit: success={}",
            response.success,
        );

        // Step 12: Validate and extract GatewayData
        if !response.success {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            tracing::warn!("Exit gateway rejected registration: {}", error_msg);
            return Err(LpClientError::RegistrationRejected { reason: error_msg });
        }

        // Extract gateway_data
        let gateway_data = response.gateway_data.ok_or_else(|| {
            LpClientError::Transport(
                "Gateway response missing gateway_data despite success=true".to_string(),
            )
        })?;

        tracing::info!(
            "Exit gateway registration successful! Allocated bandwidth: {} bytes",
            response.allocated_bandwidth
        );

        Ok(gateway_data)
    }

    /// Sends a packet via forwarding through the entry gateway and returns the parsed response.
    ///
    /// This helper consolidates the send/receive pattern used throughout the handshake:
    /// 1. Gets outer AEAD key from state machine (if available)
    /// 2. Serializes the packet with outer encryption
    /// 3. Forwards via entry gateway
    /// 4. Parses and returns the response
    async fn send_and_receive_via_forward(
        &self,
        outer_client: &mut LpRegistrationClient,
        state_machine: &LpStateMachine,
        packet: &LpPacket,
    ) -> Result<LpPacket> {
        let outer_key = state_machine.session().ok().and_then(|s| s.outer_aead_key());
        let packet_bytes = Self::serialize_packet(packet, outer_key.as_ref())?;
        let response_bytes = outer_client
            .send_forward_packet(
                self.exit_identity,
                self.exit_address.clone(),
                packet_bytes,
            )
            .await?;
        Self::parse_packet(&response_bytes, outer_key.as_ref())
    }

    /// Serializes an LP packet to bytes.
    ///
    /// # Arguments
    /// * `packet` - The LP packet to serialize
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Serialized packet bytes
    ///
    /// # Errors
    /// Returns an error if serialization fails
    fn serialize_packet(packet: &LpPacket, outer_key: Option<&OuterAeadKey>) -> Result<Vec<u8>> {
        let mut buf = BytesMut::new();
        // Use outer AEAD key when available (after PSK derivation)
        serialize_lp_packet(packet, &mut buf, outer_key).map_err(|e| {
            LpClientError::Transport(format!("Failed to serialize LP packet: {}", e))
        })?;
        Ok(buf.to_vec())
    }

    /// Parses an LP packet from bytes.
    ///
    /// # Arguments
    /// * `bytes` - The bytes to parse
    ///
    /// # Returns
    /// * `Ok(LpPacket)` - Parsed LP packet
    ///
    /// # Errors
    /// Returns an error if parsing fails
    fn parse_packet(bytes: &[u8], outer_key: Option<&OuterAeadKey>) -> Result<LpPacket> {
        // Use outer AEAD key when available (after PSK derivation)
        parse_lp_packet(bytes, outer_key).map_err(|e| {
            LpClientError::Transport(format!("Failed to parse LP packet: {}", e))
        })
    }
}
