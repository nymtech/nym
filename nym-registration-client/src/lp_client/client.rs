// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) registration client for direct gateway connections.

use super::config::LpRegistrationConfig;
use super::error::{LpClientError, Result};
use crate::lp_client::helpers::{LpDataDeliverExt, LpDataSendExt};
use crate::lp_client::nested_session::connection::NestedConnection;
use crate::lp_client::state_machine_helpers::{extract_forwarded_response, prepare_send_packet};
use nym_bandwidth_controller::{BandwidthTicketProvider, DEFAULT_TICKETS_TO_SPEND};
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::LpSession;
use nym_lp::peer::{DHKeyPair, LpLocalPeer, LpRemotePeer};
use nym_lp::state_machine::LpStateMachine;
use nym_lp::{Ciphersuite, EncryptedLpPacket, packet::version};
use nym_lp_transport::traits::LpTransportChannel;
use nym_lp_transport::{LpHandshakeChannel, LpTransportError};
use nym_registration_common::dvpn::LpDvpnRegistrationResponseMessageContent;
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationResponse, WireguardConfiguration,
    WireguardRegistrationData,
};
use nym_wireguard_types::PeerPublicKey;
use rand09::{CryptoRng, Rng, RngCore};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::warn;

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
pub struct LpRegistrationClient<S = TcpStream> {
    /// Encapsulates all the client keys needed for the Lewes Protocol.
    lp_local_peer: LpLocalPeer,

    /// Encapsulates all the gateway keys needed for the Lewes Protocol.
    gateway_lp_peer: LpRemotePeer,

    /// Gateway LP listener address (host:port, e.g., "1.1.1.1:41264").
    gateway_lp_address: SocketAddr,

    /// Supported protocol version of the remote gateway.
    /// Included in case we have to downgrade our version.
    gateway_supported_lp_protocol_version: u8,

    /// LP state machine for managing connection lifecycle.
    /// Created during handshake initiation.
    state_machine: Option<LpStateMachine>,

    /// Configuration for timeouts and TCP parameters.
    pub(crate) config: LpRegistrationConfig,

    /// Persistent TCP stream for the connection.
    /// Opened on first use, closed after registration.
    stream: Option<S>,
}

impl<S> LpRegistrationClient<S>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
{
    /// Creates a new LP registration client.
    ///
    /// # Arguments
    /// * `local_x25519_keypair` - Client's x25519 keypair
    /// * `gateway_lp_peer` - Encapsulates all the gateway keys needed for the Lewes Protocol
    /// * `gateway_lp_address` - Gateway's LP listener socket address
    /// * `ciphersuite` - the set of cryptographic protocols to use when negotiating the session with the node
    /// * `gateway_supported_lp_protocol_version` - Gateway's LP protocol version
    /// * `config` - Configuration for timeouts and TCP parameters (use `LpConfig::default()`)
    ///
    /// # Note
    /// This creates the client. Call `perform_handshake()` to establish the LP session.
    pub fn new(
        local_x25519_keypair: Arc<DHKeyPair>,
        gateway_lp_peer: LpRemotePeer,
        gateway_lp_address: SocketAddr,
        ciphersuite: Ciphersuite,
        gateway_supported_lp_protocol_version: u8,
        config: LpRegistrationConfig,
    ) -> Self {
        let lp_protocol = if gateway_supported_lp_protocol_version > version::CURRENT {
            warn!(
                "suggested LP protocol ({gateway_supported_lp_protocol_version}) is higher  than the current known version. attempting to downgrade it to {}",
                version::CURRENT
            );
            version::CURRENT
        } else {
            gateway_supported_lp_protocol_version
        };

        let lp_local_peer = LpLocalPeer::new(ciphersuite, local_x25519_keypair);
        Self {
            lp_local_peer,
            gateway_lp_peer,
            gateway_lp_address,
            gateway_supported_lp_protocol_version: lp_protocol,
            state_machine: None,
            config,
            stream: None,
        }
    }

    /// Attempt to use this `LpRegistrationClient` as transport for `NestedSession`
    pub fn as_nested_connection(&mut self, exit_address: SocketAddr) -> NestedConnection<'_, S> {
        NestedConnection {
            exit_address,
            outer_client: self,
        }
    }

    /// Creates a new LP registration client with default configuration.
    ///
    /// # Arguments
    /// * `local_x25519_keypair` - Client's x25519 keypair
    /// * `gateway_lp_peer` - Encapsulates all the gateway keys needed for the Lewes Protocol
    /// * `gateway_lp_address` - Gateway's LP listener socket address
    /// * `ciphersuite` - the set of cryptographic protocols to use when negotiating the session with the node
    /// * `gateway_supported_lp_protocol_version` - Gateway's LP protocol version
    ///
    /// Uses default config (LpConfig::default()) with sane timeout and TCP parameters.
    /// PSK is derived automatically during handshake inside the state machine.
    /// For custom config, use `new()` directly.
    pub fn new_with_default_config(
        local_x25519_keypair: Arc<DHKeyPair>,
        gateway_lp_peer: LpRemotePeer,
        gateway_lp_address: SocketAddr,
        ciphersuite: Ciphersuite,
        gateway_supported_lp_protocol_version: u8,
    ) -> Self {
        Self::new(
            local_x25519_keypair,
            gateway_lp_peer,
            gateway_lp_address,
            ciphersuite,
            gateway_supported_lp_protocol_version,
            LpRegistrationConfig::default(),
        )
    }

    pub(crate) fn state_machine(&self) -> Result<&LpStateMachine> {
        self.state_machine
            .as_ref()
            .ok_or(LpClientError::IncompleteHandshake)
    }

    pub(crate) fn state_machine_mut(&mut self) -> Result<&mut LpStateMachine> {
        self.state_machine
            .as_mut()
            .ok_or(LpClientError::IncompleteHandshake)
    }

    fn stream_mut(&mut self) -> Result<&mut S> {
        self.stream.as_mut().ok_or(LpClientError::NotConnected)
    }

    /// Returns whether the client has completed the handshake and is ready for registration.
    pub fn is_handshake_complete(&self) -> bool {
        self.state_machine.is_some()
    }

    /// Returns the gateway LP address this client is configured for.
    pub fn gateway_address(&self) -> SocketAddr {
        self.gateway_lp_address
    }

    /// Returns reference to the established connection between the client and the gateway.
    pub fn connection(&self) -> &Option<S> {
        &self.stream
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
    // Do not manually call this function. It is only exposed for the purposes of integration tests
    #[doc(hidden)]
    pub async fn ensure_connected(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Ok(());
        }

        tracing::debug!(
            "Opening persistent connection to {}",
            self.gateway_lp_address
        );

        let mut stream = tokio::time::timeout(
            self.config.connect_timeout,
            S::connect(self.gateway_lp_address),
        )
        .await
        .map_err(|_| LpClientError::TcpConnection {
            address: self.gateway_lp_address.to_string(),
            source: LpTransportError::ConnectionFailure(format!(
                "Connection timeout after {:?}",
                self.config.connect_timeout
            )),
        })?
        .map_err(|source| LpClientError::TcpConnection {
            address: self.gateway_lp_address.to_string(),
            source,
        })?;

        // Set TCP_NODELAY for low latency
        stream
            .set_no_delay(self.config.tcp_nodelay)
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

    /// Attempt to send an Lp packet on the persistent stream
    /// and attempt to immediately read a response.
    ///
    /// Both packets are going to be optionally encrypted/decrypted based on the availability of keys
    /// within the internal `LpStateMachine`
    ///
    /// # Arguments
    /// * `packet` - The LP packet to send
    ///
    /// # Errors
    /// Returns an error if not connected or if send or receive fails.
    async fn send_and_receive_packet(
        &mut self,
        packet: &EncryptedLpPacket,
    ) -> Result<EncryptedLpPacket> {
        self.try_send_packet(packet).await?;
        self.try_receive_packet().await
    }

    /// Attempt to send an Lp packet on the persistent stream
    /// and attempt to immediately read a response
    /// within the provided timeout.
    ///
    /// Both packets are going to be encrypted
    ///
    /// # Arguments
    /// * `packet` - The encrypted LP packet to send
    ///
    /// # Errors
    /// Returns an error if not connected, the timeout has been reached, or if send or receive fails.
    async fn send_and_receive_data_packet_with_timeout(
        &mut self,
        packet: &EncryptedLpPacket,
        timeout: Duration,
    ) -> Result<EncryptedLpPacket> {
        tokio::time::timeout(timeout, self.send_and_receive_packet(packet))
            .await
            .map_err(|_| LpClientError::ConnectionTimeout)?
    }

    /// Sends an LP packet on the persistent stream.
    ///
    /// # Arguments
    /// * `packet` - The LP packet to send
    ///
    /// # Errors
    /// Returns an error if not connected or if send fails.
    pub(crate) async fn try_send_packet(&mut self, packet: &EncryptedLpPacket) -> Result<()> {
        // can't use getters due to borrow checker (i.e. requiring full borrows for function calls)
        self.stream_mut()?
            .send_length_prefixed_transport_packet(packet)
            .await?;
        Ok(())
    }

    /// Receives an LP packet from the persistent stream.
    ///
    /// # Errors
    /// Returns an error if not connected or if receive fails.
    pub(crate) async fn try_receive_packet(&mut self) -> Result<EncryptedLpPacket> {
        let encrypted_packet = self
            .stream_mut()?
            .receive_length_prefixed_transport_packet()
            .await?;

        Ok(encrypted_packet)
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
        // Apply handshake timeout
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
                Err(LpClientError::HandshakeTimeout)
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

        let local_peer = self.lp_local_peer.clone();
        let remote_peer = self.gateway_lp_peer.clone();
        let protocol_version = self.gateway_supported_lp_protocol_version;
        let connection = self.stream_mut()?;

        // TODO:
        let session = LpSession::psq_handshake_initiator(
            connection,
            local_peer,
            remote_peer,
            protocol_version,
        )
        .complete_handshake()
        .await?;

        // Store the state machine (with established session) for later use
        self.state_machine = Some(LpStateMachine::new(session));
        Ok(())
    }

    /// This is an internal method only meant to be called by `Self::register_dvpn` if the gateway
    /// responds with a credential request. This is expected in every initial interaction with a particular gateway.
    ///
    /// This method will actually attempt to retrieve a valid credential from the `bandwidth_controller`
    ///
    /// # Arguments
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
    /// - Network communication fails
    /// - Gateway rejected the registration
    /// - Response times out (see LpConfig::registration_timeout)
    async fn finalise_dvpn_registration(
        &mut self,
        gateway_identity: ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
    ) -> Result<WireguardRegistrationData> {
        tracing::debug!("Acquiring bandwidth credential for registration");

        // 1. Get bandwidth credential from controller
        let credential_spending = bandwidth_controller
            .get_ecash_ticket(ticket_type, gateway_identity, DEFAULT_TICKETS_TO_SPEND)
            .await
            .map_err(|e| {
                LpClientError::SendRegistrationRequest(format!(
                    "Failed to acquire bandwidth credential: {e}",
                ))
            })?
            .data;

        // 2. Build registration request

        // for now we do NOT support upgrade mode (yeah... no.)
        let credential = credential_spending
            .try_into()
            .map_err(|err| LpClientError::Other(format!("malformed stored credential: {err}")))?;

        let request = LpRegistrationRequest::new_finalise_dvpn(credential);

        tracing::trace!("Built dVPN registration finalisation request");

        // 3. Serialize the request
        let lp_data = request.to_lp_data()?;

        // 4. Encrypt and prepare packet via state machine
        let state_machine = self.state_machine_mut()?;
        let request_packet = prepare_send_packet(lp_data, state_machine)?;

        // 5. Send initial request and receive response on persistent connection with timeout
        let response_packet = self
            .send_and_receive_data_packet_with_timeout(
                &request_packet,
                self.config.registration_timeout,
            )
            .await?;

        // 6. Decrypt via state machine (re-borrow)
        let state_machine = self.state_machine_mut()?;
        let received_data = extract_forwarded_response(response_packet, state_machine)?;

        // 7. Extract decrypted data and deserialise the response
        let response = LpRegistrationResponse::from_lp_data(received_data)?;
        let Some(dvpn_response) = response.into_dvpn_response() else {
            return Err(LpClientError::unexpected_response(
                "did not get a dvpn registration response after sending initial request",
            ));
        };

        // 8. check response to the initial request
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

    /// This is the primary registration method. It acquires a bandwidth credential,
    /// sends the registration request, and receives the response
    /// on the same underlying connection.
    /// Do note that this method does **not** perform retries on network failures,
    /// for that please use [`Self::register_with_retry`] instead
    ///
    /// # Arguments
    /// * `rng` - RNG instance for generating PSK
    /// * `wg_keypair` - Client's WireGuard x25519 keypair
    /// * `gateway_identity` - Gateway's ed25519 identity for credential verification
    /// * `bandwidth_controller` - Provider for bandwidth credentials
    /// * `ticket_type` - Type of bandwidth ticket to use
    ///
    /// # Returns
    /// * `Ok(WireguardConfiguration)` - Gateway configuration data on successful registration
    ///
    /// # Errors
    /// Returns an error if:
    /// - Handshake has not been completed
    /// - Credential acquisition fails
    /// - Request serialization/encryption fails
    /// - Network communication fails
    /// - Gateway rejected the registration
    /// - Response times out (see LpConfig::registration_timeout)
    pub async fn register_dvpn<R>(
        &mut self,
        rng: &mut R,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
    ) -> Result<WireguardConfiguration>
    where
        R: RngCore + CryptoRng,
    {
        // 1. Build registration request
        let wg_public_key = PeerPublicKey::from(*wg_keypair.public_key());
        let mut psk = [0u8; 32];
        rng.fill_bytes(&mut psk);
        let request = LpRegistrationRequest::new_initial_dvpn(wg_public_key, psk);

        tracing::trace!("Built dVPN registration request: {request:?}");

        // 2. Serialize the request
        let lp_data = request.to_lp_data()?;

        // 3. Encrypt and prepare packet via state machine
        let state_machine = self.state_machine_mut()?;
        let request_packet = prepare_send_packet(lp_data, state_machine)?;

        // 4. Send initial request and receive response on persistent connection with timeout
        let response_packet = self
            .send_and_receive_data_packet_with_timeout(
                &request_packet,
                self.config.registration_timeout,
            )
            .await?;

        // 5. Decrypt via state machine (re-borrow)
        let state_machine = self.state_machine_mut()?;
        let received_data = extract_forwarded_response(response_packet, state_machine)?;

        // 6. Extract decrypted data and deserialise the response
        let response = LpRegistrationResponse::from_lp_data(received_data)?;
        let Some(dvpn_response) = response.into_dvpn_response() else {
            return Err(LpClientError::unexpected_response(
                "did not get a dvpn registration response after sending initial request",
            ));
        };

        // 7. check response to the initial request
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

                // 8. retrieve credential from the controller
                self.finalise_dvpn_registration(
                    *gateway_identity,
                    bandwidth_controller,
                    ticket_type,
                )
                .await?
            }
        };

        Ok(WireguardConfiguration {
            public_key: final_response.public_key,
            psk: Some(psk),
            endpoint: SocketAddr::new(self.gateway_lp_address.ip(), final_response.port),
            private_ipv4: final_response.private_ipv4,
            private_ipv6: final_response.private_ipv6,
        })
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
    /// * `rng` - RNG instance for generating PSK
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
    pub async fn register_with_retry<R>(
        &mut self,
        rng: &mut R,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_controller: &dyn BandwidthTicketProvider,
        ticket_type: TicketType,
        max_retries: u32,
    ) -> Result<WireguardConfiguration>
    where
        R: RngCore + CryptoRng,
    {
        tracing::debug!("Starting resilient registration (max_retries={max_retries})",);

        let mut last_error = None;
        for attempt in 0..=max_retries {
            let attempt_display = attempt + 1;

            if attempt > 0 {
                // Exponential backoff with jitter: 100ms, 200ms, 400ms, 800ms, 1600ms (capped)
                let base_delay_ms = 100u64 * (1 << attempt.min(4));
                let jitter_ms: u64 = rand09::rng().random_range(0..(base_delay_ms / 4 + 1));
                let delay = std::time::Duration::from_millis(base_delay_ms + jitter_ms);
                tracing::info!("Retrying registration (attempt {attempt_display}) after {delay:?}");
                tokio::time::sleep(delay).await;
            }

            // Ensure fresh connection and handshake for each attempt
            // (On retry, the old connection/session may be dead)
            if self.stream.is_none() || attempt > 0 {
                // Clear any stale state before re-handshaking
                self.close();
                self.state_machine = None;

                if let Err(e) = self.perform_handshake().await {
                    tracing::warn!("Handshake failed on attempt {attempt_display}: {e}");
                    last_error = Some(e);
                    continue;
                }
            }

            match self
                .register_dvpn(
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
                        tracing::info!("Registration succeeded on retry attempt {attempt_display}");
                    }
                    return Ok(data);
                }
                Err(e) => {
                    tracing::warn!("Registration attempt {attempt_display} failed: {e}");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(LpClientError::RegistrationFailure {
            message: "Registration failed after all retries".to_string(),
        }))
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
        self.state_machine()?
            .session()
            .map(|s| s.receiver_index())
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_kkt::key_utils::generate_lp_keypair_x25519;
    use nym_lp::packet::version;
    use nym_test_utils::helpers::deterministic_rng_09;

    #[test]
    fn test_client_creation() {
        let mut rng09 = deterministic_rng_09();
        let keypair = Arc::new(generate_lp_keypair_x25519(&mut rng09));

        let gateway_x_keys = generate_lp_keypair_x25519(&mut rng09);
        let gateway_peer = LpRemotePeer::from(gateway_x_keys.pk);
        let address = "127.0.0.1:41264".parse().unwrap();

        let client = LpRegistrationClient::<TcpStream>::new_with_default_config(
            keypair,
            gateway_peer,
            address,
            Ciphersuite::default(),
            version::CURRENT,
        );

        assert!(!client.is_handshake_complete());
        assert_eq!(client.gateway_address(), address);
    }
}
