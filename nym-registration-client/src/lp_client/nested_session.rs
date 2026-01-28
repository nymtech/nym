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
use crate::lp_client::helpers::{LpDataDeliverExt, LpDataSendExt};
use crate::lp_client::state_machine_helpers::{
    extract_forwarded_response, get_recv_key, get_send_key, prepare_serialised_send_packet,
    serialize_packet,
};
use nym_bandwidth_controller::{BandwidthTicketProvider, DEFAULT_TICKETS_TO_SPEND};
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::codec::{OuterAeadKey, parse_lp_packet};
use nym_lp::message::ForwardPacketData;
use nym_lp::peer::{LpLocalPeer, LpRemotePeer};
use nym_lp::state_machine::{LpAction, LpData, LpInput, LpStateMachine};
use nym_lp::{LpMessage, LpPacket};
use nym_lp_transport::traits::LpTransport;
use nym_registration_common::dvpn::LpDvpnRegistrationResponseMessageContent;
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationResponse, WireguardConfiguration,
};
use nym_wireguard_types::PeerPublicKey;
use rand::{CryptoRng, RngCore};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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
    /// Exit gateway's LP address (e.g., "2.2.2.2:41264")
    exit_address: String,

    /// Encapsulates all the client keys needed for the Lewes Protocol.
    lp_local_peer: LpLocalPeer,

    /// Encapsulates all the exit gateway keys needed for the Lewes Protocol.
    gateway_lp_peer: LpRemotePeer,

    /// LP state machine for exit gateway session (populated after handshake)
    state_machine: Option<LpStateMachine>,
}

impl NestedLpSession {
    /// Creates a new nested LP session handler.
    ///
    /// # Arguments
    /// * `exit_address` - Exit gateway's LP address (e.g., "2.2.2.2:41264")
    /// * `client_keypair` - Client's Ed25519 keypair
    /// * `gateway_lp_peer` - Encapsulates all the gateway keys needed for the Lewes Protocol
    pub fn new(
        exit_address: String,
        client_keypair: Arc<ed25519::KeyPair>,
        gateway_lp_peer: LpRemotePeer,
    ) -> Self {
        let local_x25519_keypair = client_keypair.to_x25519();
        let lp_local_peer = LpLocalPeer::new(client_keypair, Arc::new(local_x25519_keypair));

        Self {
            exit_address,
            lp_local_peer,
            gateway_lp_peer,
            state_machine: None,
        }
    }

    fn state_machine(&self) -> Result<&LpStateMachine> {
        self.state_machine.as_ref().ok_or_else(|| {
            LpClientError::transport(
                "State machine not available - has the handshake been completed?",
            )
        })
    }

    fn state_machine_mut(&mut self) -> Result<&mut LpStateMachine> {
        self.state_machine.as_mut().ok_or_else(|| {
            LpClientError::transport(
                "State machine not available - has the handshake been completed?",
            )
        })
    }

    /// Attempt to parse received bytes into an LpPacket
    fn parse_received_lp_packet(&self, response_bytes: Vec<u8>) -> Result<LpPacket> {
        let state_machine = self.state_machine()?;
        let outer_key = get_recv_key(state_machine);
        Self::parse_packet(&response_bytes, outer_key.as_ref())
    }

    /// Attempt to wrap the provided `LpData` into a `ForwardPacketData`
    /// using the inner state machine.
    fn prepare_forward_packet(&mut self, data: LpData) -> Result<ForwardPacketData> {
        let target_gateway_identity = self.gateway_lp_peer.ed25519();
        let target_lp_address = self.exit_address.clone();

        let state_machine = self.state_machine_mut()?;
        let inner_packet_bytes = prepare_serialised_send_packet(data, state_machine)?;
        Ok(ForwardPacketData {
            target_gateway_identity: target_gateway_identity.to_bytes(),
            target_lp_address,
            inner_packet_bytes,
        })
    }

    /// Attempt to recover received `LpData` from the received `LpPacket`
    /// using the inner state machine.
    fn extract_forwarded_response(&mut self, response_packet: LpPacket) -> Result<LpData> {
        let state_machine = self.state_machine_mut()?;
        extract_forwarded_response(response_packet, state_machine)
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
    async fn perform_handshake<S>(
        &mut self,
        outer_client: &mut LpRegistrationClient<S>,
    ) -> Result<()>
    where
        S: LpTransport + Unpin,
    {
        tracing::debug!(
            "Starting nested LP handshake with exit gateway {}",
            self.exit_address
        );

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| LpClientError::Other("System time before UNIX epoch".into()))?
            .as_secs();

        // Step 1: Generate ClientHello for exit gateway
        let client_hello_data = self.lp_local_peer.build_client_hello_data(timestamp);
        let salt = client_hello_data.salt;
        let receiver_index = client_hello_data.receiver_index;

        tracing::trace!(
            "Generated ClientHello for exit gateway (timestamp: {})",
            client_hello_data.extract_timestamp()
        );

        // Step 2: Send ClientHello to exit gateway via forwarding
        let client_hello_header = nym_lp::packet::LpHeader::new(
            nym_lp::BOOTSTRAP_RECEIVER_IDX, // Use constant for bootstrap session
            0,                              // counter starts at 0
        );
        let client_hello_packet = nym_lp::LpPacket::new(
            client_hello_header,
            LpMessage::ClientHello(client_hello_data),
        );

        // Serialize and forward ClientHello (no state machine yet, no outer key)
        let client_hello_bytes = serialize_packet(&client_hello_packet, None)?;
        let forward_packet_data = ForwardPacketData::new(
            self.gateway_lp_peer.ed25519(),
            self.exit_address.clone(),
            client_hello_bytes,
        );

        let response_bytes = outer_client
            .send_forward_packet_with_response(forward_packet_data)
            .await?;

        // Parse and validate Ack response (cleartext, no outer key before PSK derivation)
        let ack_response = Self::parse_packet(&response_bytes, None)?;
        match ack_response.message() {
            LpMessage::Ack => {
                tracing::debug!("Received Ack for ClientHello from exit gateway");
            }
            LpMessage::Collision => {
                return Err(LpClientError::Transport(format!(
                    "Exit gateway returned Collision - receiver_index {receiver_index} already in use",
                )));
            }
            other => {
                return Err(LpClientError::Transport(format!(
                    "Expected Ack for ClientHello from exit gateway, got: {:?}",
                    other
                )));
            }
        }

        // Step 3: Create state machine for exit gateway handshake
        let mut state_machine = LpStateMachine::new(
            receiver_index,
            true, // is_initiator
            self.lp_local_peer.clone(),
            self.gateway_lp_peer.clone(),
            &salt,
        )?;

        // Step 4: Get initial packet from StartHandshake
        let mut pending_packet: Option<LpPacket> = None;
        if let Some(action) = state_machine.process_input(LpInput::StartHandshake) {
            match action? {
                LpAction::SendPacket(packet) => {
                    pending_packet = Some(packet);
                }
                other => {
                    return Err(LpClientError::Transport(format!(
                        "Unexpected action at handshake start: {other:?}",
                    )));
                }
            }
        }

        // Step 5: Handshake loop - each packet on new connection via forwarding
        loop {
            if let Some(packet) = pending_packet.take() {
                tracing::trace!("Sending handshake packet to exit via forwarding");
                let response = self
                    .send_and_receive_via_forward(outer_client, &state_machine, &packet)
                    .await?;
                tracing::trace!("Received handshake response from exit");

                // Process the received packet
                if let Some(action) = state_machine.process_input(LpInput::ReceivePacket(response))
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
                                tracing::info!("Nested LP handshake completed with exit gateway");
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

    /// This is an internal method only meant to be called by `Self::handshake_and_register_dvpn` if the gateway
    /// responds with a credential request. This is expected in every initial interaction with a particular gateway.
    ///
    /// This method will actually attempt to retrieve a valid credential from the `bandwidth_controller`
    ///
    /// # Arguments
    /// * `outer_client` - Connected LP client with established outer session to entry gateway
    /// * `gateway_identity` - Gateway's ed25519 identity for credential verification
    /// * `bandwidth_controller` - Provider for bandwidth credentials
    /// * `ticket_type` - Type of bandwidth ticket to use
    ///
    /// # Returns
    /// * `Ok(WireguardConfiguration)` - Gateway configuration data on successful registration
    ///
    /// # Errors
    /// Returns an error if:
    /// - Credential acquisition fails
    /// - Request serialization/encryption fails
    /// - Forwarding through entry gateway fails
    /// - Network communication fails
    /// - Gateway rejected the registration
    /// - Response times out (see LpConfig::registration_timeout)
    async fn finalise_dvpn_registration<S>(
        &mut self,
        outer_client: &mut LpRegistrationClient<S>,
        gateway_identity: ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
    ) -> Result<WireguardConfiguration>
    where
        S: LpTransport + Unpin,
    {
        tracing::debug!("Acquiring bandwidth credential for registration");

        // Step 1: Get bandwidth credential from controller
        let credential_spending = bandwidth_controller
            .get_ecash_ticket(ticket_type, gateway_identity, DEFAULT_TICKETS_TO_SPEND)
            .await
            .map_err(|e| {
                LpClientError::SendRegistrationRequest(format!(
                    "Failed to acquire bandwidth credential: {e}",
                ))
            })?
            .data;

        // Step 2: Build registration request

        // for now we do NOT support upgrade mode (yeah... no.)
        let credential = credential_spending
            .try_into()
            .map_err(|err| LpClientError::Other(format!("malformed stored credential: {err}")))?;

        let request = LpRegistrationRequest::new_finalise_dvpn(credential);

        tracing::trace!("Built dVPN registration finalisation request");

        // Step 3: Serialize the request
        let send_data = request.to_lp_data()?;

        // Step 4: Encrypt and prepare packet via state machine
        let forward_packet = self.prepare_forward_packet(send_data)?;

        // Step 5: Send the encrypted packet via forwarding
        let response_bytes = outer_client
            .send_forward_packet_with_response(forward_packet)
            .await?;

        // Step 6: Parse response bytes to LP packet
        let response_packet = self.parse_received_lp_packet(response_bytes)?;

        // Step 7: Decrypt via state machine
        let response_data = self.extract_forwarded_response(response_packet)?;

        // Step 8: Extract decrypted data and deserialise the response
        let response = LpRegistrationResponse::from_lp_data(response_data)?;
        let Some(dvpn_response) = response.into_dvpn_response() else {
            return Err(LpClientError::unexpected_response(
                "did not get a dvpn registration response after sending initial request",
            ));
        };

        // Step 9: check response to the initial request
        match dvpn_response.content {
            LpDvpnRegistrationResponseMessageContent::RegistrationFailure(res) => {
                let reason = res.error;
                // the registration has failed
                tracing::warn!("Gateway rejected registration: {reason}");
                Err(LpClientError::RegistrationRejected { reason })
            }
            LpDvpnRegistrationResponseMessageContent::CompletedRegistration(res) => {
                // we have managed to complete the registration
                tracing::info!(
                    "LP registration successful! Allocated bandwidth: {} bytes",
                    res.available_bandwidth
                );
                Ok(res.config)
            }
            LpDvpnRegistrationResponseMessageContent::RequiresCredential(_) => {
                Err(LpClientError::unexpected_response(
                    "received request for additional dvpn data after sending credential!",
                ))
            }
        }
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
    pub async fn handshake_and_register_dvpn<S, R>(
        &mut self,
        outer_client: &mut LpRegistrationClient<S>,
        rng: &mut R,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
    ) -> Result<WireguardConfiguration>
    where
        S: LpTransport + Unpin,
        R: RngCore + CryptoRng,
    {
        // Step 1: Perform handshake with exit gateway via forwarding
        self.perform_handshake(outer_client).await?;

        tracing::debug!("Building registration request for exit gateway");

        // Step 2: Build registration request
        let wg_public_key = PeerPublicKey::from(*wg_keypair.public_key());
        let request = LpRegistrationRequest::new_initial_dvpn(rng, wg_public_key, ticket_type);

        // Step 3: Serialize the request
        let send_data = request.to_lp_data()?;

        // Step 4: Encrypt and prepare packet via state machine
        let forward_packet = self.prepare_forward_packet(send_data)?;

        // Step 5: Send the encrypted packet via forwarding
        let response_bytes = outer_client
            .send_forward_packet_with_response(forward_packet)
            .await?;

        tracing::trace!("Received registration response from exit gateway");

        // Step 6: Parse response bytes to LP packet
        let response_packet = self.parse_received_lp_packet(response_bytes)?;

        // Step 7: Decrypt via state machine
        let response_data = self.extract_forwarded_response(response_packet)?;

        // Step 8: Extract decrypted data and deserialise the response
        let response = LpRegistrationResponse::from_lp_data(response_data)?;
        let Some(dvpn_response) = response.into_dvpn_response() else {
            return Err(LpClientError::unexpected_response(
                "did not get a dvpn registration response after sending initial request",
            ));
        };

        // Step 9: check response to the initial request
        match dvpn_response.content {
            LpDvpnRegistrationResponseMessageContent::RegistrationFailure(res) => {
                let reason = res.error;
                // the registration has failed
                tracing::warn!("Gateway rejected registration: {reason}");
                Err(LpClientError::RegistrationRejected { reason })
            }
            LpDvpnRegistrationResponseMessageContent::CompletedRegistration(res) => {
                // we have already registered with this gateway before, the gateway has updated the psk and sent us the config
                tracing::info!(
                    "LP registration successful! Allocated bandwidth: {} bytes",
                    res.available_bandwidth
                );
                Ok(res.config)
            }
            LpDvpnRegistrationResponseMessageContent::RequiresCredential(_) => {
                // we're registering for the first time with this gateway - we need to attach a credential

                // Step 10: retrieve credential from the controller
                self.finalise_dvpn_registration(
                    outer_client,
                    *gateway_identity,
                    bandwidth_controller,
                    ticket_type,
                )
                .await
            }
        }
    }

    /// Performs handshake and registration with the exit gateway via forwarding,
    /// with automatic retry on network failure.
    ///
    /// This method:
    /// 1. Acquires credential ONCE
    /// 2. Performs handshake and registration with exit gateway
    /// 3. On network failure, clears state and retries with same credential
    /// 4. Gateway idempotency ensures no double-spend even if credential was processed
    ///
    /// Use this method for resilient exit registration on unreliable networks (e.g., train
    /// through tunnel). The gateway's idempotent registration check ensures that if
    /// a registration succeeds but the response is lost, retrying with the same WG key
    /// will return the cached result instead of spending a new credential.
    ///
    /// # Arguments
    /// * `outer_client` - Connected LP client with established outer session to entry gateway
    /// * `wg_keypair` - Client's WireGuard x25519 keypair (same key used for all retries)
    /// * `gateway_identity` - Exit gateway's Ed25519 identity (for credential verification)
    /// * `bandwidth_controller` - Provider for bandwidth credentials
    /// * `ticket_type` - Type of bandwidth ticket to use
    /// * `client_ip` - Client IP address for registration metadata
    /// * `max_retries` - Maximum number of retry attempts after initial failure
    ///
    /// # Returns
    /// * `Ok(GatewayData)` - Exit gateway configuration data on successful registration
    ///
    /// # Errors
    /// Returns an error if all retry attempts fail.
    #[allow(clippy::too_many_arguments)]
    pub async fn handshake_and_register_with_retry<S, R>(
        &mut self,
        rng: &mut R,
        outer_client: &mut LpRegistrationClient<S>,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
        max_retries: u32,
    ) -> Result<WireguardConfiguration>
    where
        S: LpTransport + Unpin,
        R: RngCore + CryptoRng,
    {
        tracing::debug!(
            "Starting resilient exit registration (max_retries={})",
            max_retries
        );

        let mut last_error = None;
        for attempt in 0..=max_retries {
            if attempt > 0 {
                // Verify outer session is still usable before retry
                if !outer_client.is_handshake_complete() {
                    return Err(LpClientError::Transport(
                        "Outer session lost during retry - caller must re-establish entry gateway connection".to_string()
                    ));
                }

                // Exponential backoff with jitter: 100ms, 200ms, 400ms, 800ms, 1600ms (capped)
                let base_delay_ms = 100u64 * (1 << attempt.min(4));
                let jitter_ms = rand::random::<u64>() % (base_delay_ms / 4 + 1);
                let delay = std::time::Duration::from_millis(base_delay_ms + jitter_ms);
                tracing::info!(
                    "Retrying exit registration (attempt {}) after {:?}",
                    attempt + 1,
                    delay
                );
                tokio::time::sleep(delay).await;

                // Clear state machine before retry - handshake needs fresh start
                self.state_machine = None;
            }

            match self
                .handshake_and_register_dvpn(
                    outer_client,
                    rng,
                    wg_keypair,
                    gateway_identity,
                    bandwidth_controller,
                    ticket_type,
                )
                .await
            {
                Ok(data) => {
                    if attempt > 0 {
                        tracing::info!(
                            "Exit registration succeeded on retry attempt {}",
                            attempt + 1
                        );
                    }
                    return Ok(data);
                }
                Err(e) => {
                    tracing::warn!("Exit registration attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            LpClientError::Transport("Exit registration failed after all retries".to_string())
        }))
    }

    /// Sends a packet via forwarding through the entry gateway and returns the parsed response.
    ///
    /// This helper consolidates the send/receive pattern used throughout the handshake:
    /// 1. Gets outer AEAD key from state machine (if available)
    /// 2. Serializes the packet with outer encryption
    /// 3. Forwards via entry gateway
    /// 4. Parses and returns the response
    async fn send_and_receive_via_forward<S>(
        &self,
        outer_client: &mut LpRegistrationClient<S>,
        state_machine: &LpStateMachine,
        packet: &LpPacket,
    ) -> Result<LpPacket>
    where
        S: LpTransport + Unpin,
    {
        let send_key = get_send_key(state_machine);
        let packet_bytes = serialize_packet(packet, send_key.as_ref())?;
        let forward_data = ForwardPacketData::new(
            self.gateway_lp_peer.ed25519(),
            self.exit_address.clone(),
            packet_bytes,
        );
        let response_bytes = outer_client
            .send_forward_packet_with_response(forward_data)
            .await?;
        let recv_key = get_recv_key(state_machine);
        Self::parse_packet(&response_bytes, recv_key.as_ref())
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
        parse_lp_packet(bytes, outer_key)
            .map_err(|e| LpClientError::Transport(format!("Failed to parse LP packet: {e}")))
    }
}
