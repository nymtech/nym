// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) channel to a single gateway.

use super::config::LpGatewayClientConfig;
use super::error::{LpClientError, Result};
use crate::nested_session::connection::NestedConnection;
use nym_lp::Ciphersuite;
use nym_lp::LpTransportSession;
use nym_lp::peer::{DHKeyPair, LpLocalPeer, LpRemotePeer};
use nym_lp::psq::initiator::HandshakeMode;
use nym_lp::transport::traits::LpTransportChannel;
use nym_lp::transport::{LpHandshakeChannel, LpTransportError};
use nym_lp_data::packet::{EncryptedLpPacket, header::LpReceiverIndex, version};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::warn;

/// A client's LP channel to one gateway.
///
/// Owns a single connection, opened on first use, and the session established over it. The
/// connection stays open after the handshake so callers can keep sending on it - including
/// [`NestedLpSession`](crate::NestedLpSession), which tunnels a second handshake through it.
///
/// # Example Flow
/// ```ignore
/// let mut client = LpGatewayClient::new(...);
/// client.perform_handshake().await?;   // KKT/PSQ handshake over the connection
/// // ... send frames on the session ...
/// client.close();
/// ```
pub struct LpGatewayClient<S = TcpStream> {
    /// Encapsulates all the client keys needed for the Lewes Protocol.
    lp_local_peer: LpLocalPeer,

    /// Encapsulates all the gateway keys needed for the Lewes Protocol.
    gateway_lp_peer: LpRemotePeer,

    /// Gateway LP listener address (host:port, e.g., "1.1.1.1:41264").
    gateway_lp_address: SocketAddr,

    /// Supported protocol version of the remote gateway.
    /// Included in case we have to downgrade our version.
    gateway_supported_lp_protocol_version: u8,

    /// LP transport session
    /// Created during handshake initiation.
    transport_session: Option<LpTransportSession>,

    /// Configuration for timeouts and TCP parameters.
    pub config: LpGatewayClientConfig,

    /// Persistent TCP stream for the connection.
    /// Opened on first use, closed after registration.
    stream: Option<S>,
}

impl<S> LpGatewayClient<S>
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
        config: LpGatewayClientConfig,
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
            transport_session: None,
            config,
            stream: None,
        }
    }

    /// Attempt to use this `LpGatewayClient` as transport for `NestedSession`
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
            LpGatewayClientConfig::default(),
        )
    }

    pub fn transport_session(&self) -> Result<&LpTransportSession> {
        self.transport_session
            .as_ref()
            .ok_or(LpClientError::IncompleteHandshake)
    }

    pub fn transport_session_mut(&mut self) -> Result<&mut LpTransportSession> {
        self.transport_session
            .as_mut()
            .ok_or(LpClientError::IncompleteHandshake)
    }

    fn stream_mut(&mut self) -> Result<&mut S> {
        self.stream.as_mut().ok_or(LpClientError::NotConnected)
    }

    /// Returns whether the client has completed the handshake and is ready for registration.
    pub fn is_handshake_complete(&self) -> bool {
        self.transport_session.is_some()
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
    pub async fn send_and_receive_data_packet_with_timeout(
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
    pub async fn try_send_packet(&mut self, packet: &EncryptedLpPacket) -> Result<()> {
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
    pub async fn try_receive_packet(&mut self) -> Result<EncryptedLpPacket> {
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

    /// Drop both the session and the connection, so the next [`Self::perform_handshake`] starts
    /// from nothing.
    ///
    /// A half-finished handshake cannot be resumed, and keeping the connection would leave the
    /// gateway holding a session this client has forgotten - so a retry begins from a clean slate
    /// on both.
    pub fn reset(&mut self) {
        self.transport_session = None;
        self.close();
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

        let session = LpTransportSession::psq_handshake_initiator(
            connection,
            local_peer,
            remote_peer,
            protocol_version,
            HandshakeMode::OneWayEntry,
        )?
        .complete_handshake()
        .await?;

        // Store the state machine (with established session) for later use
        self.transport_session = Some(session);
        Ok(())
    }

    /// Get the LP session ID (receiver_idx) for this client.
    ///
    /// This ID is included in the outer header of LP packets and is used by
    /// the gateway to look up the session for decryption.
    ///
    /// # Returns
    /// * `Ok(LpReceiverIndex)` - The session ID
    ///
    /// # Errors
    /// Returns an error if handshake has not been completed.
    pub fn session_id(&self) -> Result<LpReceiverIndex> {
        Ok(self.transport_session()?.receiver_index())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_kkt::key_utils::generate_lp_keypair_x25519;
    use nym_lp_data::packet::version;
    use nym_test_utils::helpers::deterministic_rng_09;

    #[test]
    fn test_client_creation() {
        let mut rng010 = deterministic_rng_09();
        let keypair = Arc::new(generate_lp_keypair_x25519(&mut rng010));

        let gateway_x_keys = generate_lp_keypair_x25519(&mut rng010);
        let gateway_peer = LpRemotePeer::from(gateway_x_keys.pk);
        let address = "127.0.0.1:41264".parse().unwrap();

        let client = LpGatewayClient::<TcpStream>::new_with_default_config(
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
