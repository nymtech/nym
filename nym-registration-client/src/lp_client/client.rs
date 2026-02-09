// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) registration client for direct gateway connections.

use super::config::LpRegistrationConfig;
use super::error::{LpClientError, Result};
use crate::lp_client::helpers::{
    LpDataDeliverExt, LpDataSendExt, convert_forward_data, try_convert_forward_response,
};
use crate::lp_client::nested_session::connection::NestedConnection;
use crate::lp_client::state_machine_helpers::{extract_forwarded_response, prepare_send_packet};
use bytes::BytesMut;
use nym_bandwidth_controller::{BandwidthTicketProvider, DEFAULT_TICKETS_TO_SPEND};
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::codec::{OuterAeadKey, parse_lp_packet, serialize_lp_packet};
use nym_lp::message::ForwardPacketData;
use nym_lp::packet::version;
use nym_lp::peer::{LpLocalPeer, LpRemotePeer};
use nym_lp::state_machine::{LpAction, LpData, LpInput, LpStateMachine};
use nym_lp::{LpPacket, LpSession};
use nym_lp_transport::traits::LpTransport;
use nym_registration_common::dvpn::LpDvpnRegistrationResponseMessageContent;
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationResponse, WireguardConfiguration,
    WireguardRegistrationData,
};
use nym_wireguard_types::PeerPublicKey;
use rand::{CryptoRng, RngCore};
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
    /// Created during handshake initiation. Persists across packet-per-connection calls.
    state_machine: Option<LpStateMachine>,

    /// Configuration for timeouts and TCP parameters.
    pub(crate) config: LpRegistrationConfig,

    /// Persistent TCP stream for the connection.
    /// Opened on first use, closed after registration.
    stream: Option<S>,
}

impl<S> LpRegistrationClient<S>
where
    S: LpTransport + Unpin,
{
    /// Creates a new LP registration client.
    ///
    /// # Arguments
    /// * `local_ed25519_keypair` - Client's Ed25519 identity keypair
    /// * `gateway_lp_peer` - Encapsulates all the gateway keys needed for the Lewes Protocol
    /// * `gateway_lp_address` - Gateway's LP listener socket address
    /// * `gateway_supported_lp_protocol_version` - Gateway's LP protocol version
    /// * `config` - Configuration for timeouts and TCP parameters (use `LpConfig::default()`)
    ///
    /// # Note
    /// This creates the client. Call `perform_handshake()` to establish the LP session.
    pub fn new(
        local_ed25519_keypair: Arc<ed25519::KeyPair>,
        gateway_lp_peer: LpRemotePeer,
        gateway_lp_address: SocketAddr,
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

        let local_x25519_keypair = local_ed25519_keypair.to_x25519();
        let lp_local_peer = LpLocalPeer::new(local_ed25519_keypair, Arc::new(local_x25519_keypair));
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
    pub fn as_nested_connection(
        &mut self,
        exit_identity: ed25519::PublicKey,
        exit_address: SocketAddr,
    ) -> NestedConnection<'_, S> {
        NestedConnection {
            exit_identity,
            exit_address,
            outer_client: self,
        }
    }

    /// Creates a new LP registration client with default configuration.
    ///
    /// # Arguments
    /// * `local_ed25519_keypair` - Client's Ed25519 identity keypair
    /// * `gateway_lp_peer` - Encapsulates all the gateway keys needed for the Lewes Protocol
    /// * `gateway_lp_address` - Gateway's LP listener socket address
    /// * `gateway_supported_lp_protocol_version` - Gateway's LP protocol version
    ///
    /// Uses default config (LpConfig::default()) with sane timeout and TCP parameters.
    /// PSK is derived automatically during handshake inside the state machine.
    /// For custom config, use `new()` directly.
    pub fn new_with_default_config(
        local_ed25519_keypair: Arc<ed25519::KeyPair>,
        gateway_lp_peer: LpRemotePeer,
        gateway_lp_address: SocketAddr,
        gateway_supported_lp_protocol_version: u8,
    ) -> Self {
        Self::new(
            local_ed25519_keypair,
            gateway_lp_peer,
            gateway_lp_address,
            gateway_supported_lp_protocol_version,
            LpRegistrationConfig::default(),
        )
    }

    pub(crate) fn state_machine_mut(&mut self) -> Result<&mut LpStateMachine> {
        self.state_machine.as_mut().ok_or_else(|| {
            LpClientError::transport(
                "State machine not available - has the handshake been completed?",
            )
        })
    }

    fn stream_mut(&mut self) -> Result<&mut S> {
        self.stream
            .as_mut()
            .ok_or_else(|| LpClientError::transport("Cannot send: not connected"))
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
    async fn send_and_receive_packet(&mut self, packet: &LpPacket) -> Result<LpPacket> {
        self.try_send_packet(packet).await?;
        self.try_receive_packet().await
    }

    /// Attempt to send an Lp packet on the persistent stream
    /// and attempt to immediately read a response
    /// within the provided timeout.
    ///
    /// Both packets are going to be optionally encrypted/decrypted based on the availability of keys
    /// within the internal `LpStateMachine`
    ///
    /// # Arguments
    /// * `packet` - The LP packet to send
    ///
    /// # Errors
    /// Returns an error if not connected, the timeout has been reached, or if send or receive fails.
    async fn send_and_receive_packet_with_timeout(
        &mut self,
        packet: &LpPacket,
        timeout: Duration,
    ) -> Result<LpPacket> {
        tokio::time::timeout(timeout, self.send_and_receive_packet(packet))
            .await
            .map_err(|_| LpClientError::ResponseReceiveTimeout { timeout })?
    }

    /// Sends an LP packet on the persistent stream.
    ///
    /// # Arguments
    /// * `packet` - The LP packet to send
    ///
    /// # Errors
    /// Returns an error if not connected or if send fails.
    pub(crate) async fn try_send_packet(&mut self, packet: &LpPacket) -> Result<()> {
        // can't use getters due to borrow checker (i.e. requiring full borrows for function calls)
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| LpClientError::transport("Cannot send: not connected"))?;

        let state_machine = self.state_machine.as_ref().ok_or_else(|| {
            LpClientError::transport(
                "State machine not available - has the handshake been completed?",
            )
        })?;

        let outer_key = state_machine.session()?.outer_aead_key();
        Self::send_packet_with_key(stream, packet, outer_key).await
    }

    /// Receives an LP packet from the persistent stream.
    ///
    /// # Errors
    /// Returns an error if not connected or if receive fails.
    pub(crate) async fn try_receive_packet(&mut self) -> Result<LpPacket> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| LpClientError::transport("Cannot send: not connected"))?;

        let state_machine = self.state_machine.as_ref().ok_or_else(|| {
            LpClientError::transport(
                "State machine not available - has the handshake been completed?",
            )
        })?;

        let outer_key = state_machine.session()?.outer_aead_key();

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

        let local_peer = self.lp_local_peer.clone();
        let remote_peer = self.gateway_lp_peer.clone();
        let protocol_version = self.gateway_supported_lp_protocol_version;
        let connection = self.stream_mut()?;

        // TODO:
        let ciphersuite = LpSession::default_ciphersuite();
        let session =
            LpSession::psq_handshake_state(connection, ciphersuite, local_peer, remote_peer)
                .psq_handshake_initiator(protocol_version)
                .await?;

        // Store the state machine (with established session) for later use
        self.state_machine = Some(LpStateMachine::new2(session));
        Ok(())
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
        stream: &mut S,
        packet: &LpPacket,
        outer_key: &OuterAeadKey,
    ) -> Result<()> {
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(packet, &mut packet_buf, Some(outer_key))
            .map_err(|e| LpClientError::Transport(format!("Failed to serialize packet: {e}")))?;

        stream
            .send_serialised_packet(&packet_buf)
            .await
            .map_err(|err| LpClientError::Transport(err.to_string()))
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
    async fn receive_packet_with_key(stream: &mut S, outer_key: &OuterAeadKey) -> Result<LpPacket> {
        let packet_buf = stream
            .receive_raw_packet()
            .await
            .map_err(|err| LpClientError::transport(err.to_string()))?;

        let packet = parse_lp_packet(&packet_buf, Some(outer_key))
            .map_err(|e| LpClientError::Transport(format!("Failed to parse packet: {e}")))?;

        Ok(packet)
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
            .send_and_receive_packet_with_timeout(&request_packet, self.config.registration_timeout)
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
            .send_and_receive_packet_with_timeout(&request_packet, self.config.registration_timeout)
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
                let jitter_ms = rand::random::<u64>() % (base_delay_ms / 4 + 1);
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
    /// * `forward_data` - encapsulated target gateway's ed25519 identity, socket address and serialised inner LP packet
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
    pub async fn send_forward_packet_with_response(
        &mut self,
        forward_data: ForwardPacketData,
    ) -> Result<Vec<u8>> {
        let target_address = forward_data.target_lp_address;

        tracing::debug!(
            "Sending ForwardPacket to {target_address} ({} inner bytes, persistent connection)",
            forward_data.inner_packet_bytes.len()
        );

        // 1. Serialize the ForwardPacketData
        let input = convert_forward_data(forward_data)?;

        // 2. Encrypt and prepare packet via state machine
        let state_machine = self.state_machine_mut()?;

        let action = state_machine
            .process_input(input)
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

        // 3. Send and receive on persistent connection with timeout
        let response_packet = tokio::time::timeout(self.config.forward_timeout, async {
            self.try_send_packet(&forward_packet).await?;
            self.try_receive_packet().await
        })
        .await
        .map_err(|_| {
            LpClientError::Transport(format!(
                "Forward packet timeout after {:?}",
                self.config.forward_timeout
            ))
        })??;
        tracing::trace!("Received response packet from entry gateway");

        // 4. Decrypt via state machine (re-borrow)
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

        // 5. Extract decrypted response data
        let response_data = try_convert_forward_response(action)?;

        tracing::debug!(
            "Successfully received forward response from {target_address} ({} bytes)",
            response_data.len()
        );

        Ok(response_data)
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
            .process_input(LpInput::SendData(LpData::new_opaque(data.to_vec())))
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
        let outer_key = state_machine.session()?.outer_aead_key();

        // Serialize the packet with outer AEAD encryption
        let mut buf = BytesMut::new();
        serialize_lp_packet(&packet, &mut buf, Some(outer_key))
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
    use nym_lp::packet::version;

    #[test]
    fn test_client_creation() {
        let mut rng = rand::thread_rng();
        let keypair = Arc::new(ed25519::KeyPair::new(&mut rng));
        let gateway_ed_keys = ed25519::KeyPair::new(&mut rng);
        let gateway_x_keys = gateway_ed_keys.to_x25519();
        let gateway_peer =
            LpRemotePeer::new(*gateway_ed_keys.public_key(), *gateway_x_keys.public_key());
        let address = "127.0.0.1:41264".parse().unwrap();

        let client = LpRegistrationClient::<TcpStream>::new_with_default_config(
            keypair,
            gateway_peer,
            address,
            version::CURRENT,
        );

        assert!(!client.is_handshake_complete());
        assert_eq!(client.gateway_address(), address);
    }
}
