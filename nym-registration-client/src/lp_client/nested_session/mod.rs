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
    extract_forwarded_response, prepare_serialised_send_packet,
};
use nym_bandwidth_controller::{BandwidthTicketProvider, DEFAULT_TICKETS_TO_SPEND};
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::packet::version;
use nym_lp::peer::{DHKeyPair, LpLocalPeer, LpRemotePeer};
use nym_lp::state_machine::{LpData, LpStateMachine};
use nym_lp::{ForwardPacketData, LpPacket, LpSession};
use nym_lp_transport::LpChannel;
use nym_lp_transport::traits::LpTransport;
use nym_registration_common::dvpn::LpDvpnRegistrationResponseMessageContent;
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationResponse, WireguardConfiguration,
    WireguardRegistrationData,
};
use nym_wireguard_types::PeerPublicKey;
use rand09::{Rng, CryptoRng, RngCore};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::warn;

pub(crate) mod connection;

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
    exit_address: SocketAddr,

    /// Encapsulates all the client keys needed for the Lewes Protocol.
    lp_local_peer: LpLocalPeer,

    /// Encapsulates all the exit gateway keys needed for the Lewes Protocol.
    gateway_lp_peer: LpRemotePeer,

    /// Supported protocol version of the remote gateway.
    /// Included in case we have to downgrade our version.
    gateway_supported_lp_protocol_version: u8,

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
    /// * `gateway_supported_lp_protocol_version` - Gateway's LP protocol version
    pub fn new(
        exit_address: SocketAddr,
        client_keypair: Arc<DHKeyPair>,
        gateway_lp_peer: LpRemotePeer,
        gateway_supported_lp_protocol_version: u8,
    ) -> Self {
        todo!()
        // let local_x25519_keypair = client_keypair.to_x25519();
        // let lp_local_peer = LpLocalPeer::new(client_keypair, Arc::new(local_x25519_keypair));
        //
        // let lp_protocol = if gateway_supported_lp_protocol_version > version::CURRENT {
        //     warn!(
        //         "suggested LP protocol ({gateway_supported_lp_protocol_version}) is higher  than the current known version. attempting to downgrade it to {}",
        //         version::CURRENT
        //     );
        //     version::CURRENT
        // } else {
        //     gateway_supported_lp_protocol_version
        // };
        //
        // Self {
        //     exit_address,
        //     lp_local_peer,
        //     gateway_lp_peer,
        //     gateway_supported_lp_protocol_version: lp_protocol,
        //     state_machine: None,
        // }
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
        todo!()
        // Self::parse_packet(&response_bytes, Some(outer_key))
    }

    /// Attempt to wrap the provided `LpData` into a `ForwardPacketData`
    /// using the inner state machine.
    fn prepare_forward_packet(&mut self, data: LpData) -> Result<ForwardPacketData> {
        let state_machine = self.state_machine_mut()?;
        let inner_packet_bytes = prepare_serialised_send_packet(data, state_machine)?;
        todo!()
        // Ok(ForwardPacketData::new(
        //     self.gateway_lp_peer.ed25519(),
        //     self.exit_address,
        //     inner_packet_bytes,
        // ))
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
        S: LpTransport + LpChannel + Unpin,
    {
        tracing::debug!(
            "Starting nested LP handshake with exit gateway {}",
            self.exit_address
        );

        todo!()
        // let mut nested_connection =
        //     outer_client.as_nested_connection(self.gateway_lp_peer.ed25519(), self.exit_address);
        //
        // let local_peer = self.lp_local_peer.clone();
        // let remote_peer = self.gateway_lp_peer.clone();
        // let protocol_version = self.gateway_supported_lp_protocol_version;
        //
        // let ciphersuite = LpSession::default_ciphersuite();
        // todo!()
        // let session = LpSession::psq_handshake_initiator(
        //     &mut nested_connection,
        //     ciphersuite,
        //     local_peer,
        //     remote_peer,
        //     protocol_version,
        // )
        // .complete_as_initiator()
        // .await?;
        //
        // // Store the state machine (with established session) for later use
        // self.state_machine = Some(LpStateMachine::new(session));
        // Ok(())
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
    ) -> Result<WireguardRegistrationData>
    where
        S: LpTransport + LpChannel + Unpin,
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
            LpDvpnRegistrationResponseMessageContent::CompletedRegistration(res) => Ok(res.config),
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
        S: LpTransport + LpChannel + Unpin,
        R: RngCore + CryptoRng,
    {
        // Step 1: Perform handshake with exit gateway via forwarding
        self.perform_handshake(outer_client).await?;

        tracing::debug!("Building registration request for exit gateway");

        // Step 2: Build registration request
        let wg_public_key = PeerPublicKey::from(*wg_keypair.public_key());
        let mut psk = [0u8; 32];
        rng.fill_bytes(&mut psk);

        let request = LpRegistrationRequest::new_initial_dvpn(wg_public_key, psk);

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
        let final_response = match dvpn_response.content {
            LpDvpnRegistrationResponseMessageContent::RegistrationFailure(res) => {
                let reason = res.error;
                // the registration has failed
                tracing::warn!("Gateway rejected registration: {reason}");
                return Err(LpClientError::RegistrationRejected { reason });
            }
            LpDvpnRegistrationResponseMessageContent::CompletedRegistration(res) => res.config,
            LpDvpnRegistrationResponseMessageContent::RequiresCredential(_) => {
                // we're registering for the first time with this gateway - we need to attach a credential

                // Step 10: retrieve credential from the controller
                self.finalise_dvpn_registration(
                    outer_client,
                    *gateway_identity,
                    bandwidth_controller,
                    ticket_type,
                )
                .await?
            }
        };

        // JS/SW TODO Adapt this to new gateway response
        Ok(WireguardConfiguration {
            public_key: final_response.public_key,
            psk: Some(psk),
            endpoint: SocketAddr::new(self.exit_address.ip(), final_response.port),
            private_ipv4: final_response.private_ipv4,
            private_ipv6: final_response.private_ipv6,
        })
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
    pub async fn handshake_and_register_dvpn_with_retry<S, R>(
        &mut self,
        outer_client: &mut LpRegistrationClient<S>,
        rng: &mut R,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
        max_retries: u32,
    ) -> Result<WireguardConfiguration>
    where
        S: LpTransport + LpChannel + Unpin,
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
                let jitter_ms: u64 = rand09::rng().random_range(0..(base_delay_ms / 4 + 1));
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
}
