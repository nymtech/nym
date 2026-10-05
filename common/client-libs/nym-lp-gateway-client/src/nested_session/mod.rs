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

use super::client::LpGatewayClient;
use super::error::{LpClientError, Result};
use crate::session_helpers::{extract_forwarded_response, prepare_send_packet};
use nym_lp::peer::{DHKeyPair, LpLocalPeer, LpRemotePeer};
use nym_lp::psq::initiator::HandshakeMode;
use nym_lp::transport::LpHandshakeChannel;
use nym_lp::transport::traits::LpTransportChannel;
use nym_lp::{Ciphersuite, KEM, LpTransportSession};
use nym_lp_data::packet::version;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, warn};

pub mod connection;

/// A session with an exit gateway, reached by forwarding packets through an entry gateway.
///
/// Has no connection of its own: an established [`LpGatewayClient`] carries its frames, so that
/// channel has to stay alive and is passed back in on every exchange - starting with
/// [`Self::perform_handshake`].
///
/// # Example
///
/// ```ignore
/// // Outer session already established with entry gateway
/// let mut outer_client = LpGatewayClient::new(...);
/// outer_client.perform_handshake().await?;
///
/// // Now establish inner session with exit gateway
/// let mut nested = NestedLpSession::new(
///     exit_address,
///     client_keypair,
///     exit_peer,
///     ciphersuite,
///     exit_lp_protocol,
/// );
///
/// nested.perform_handshake(&mut outer_client).await?;
/// // ... send frames on the nested session, forwarded by the entry gateway ...
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

    /// LP transport session for exit gateway session (populated after handshake)
    transport_session: Option<LpTransportSession>,
}

impl NestedLpSession {
    /// Creates a new nested LP session handler.
    ///
    /// # Arguments
    /// * `exit_address` - Exit gateway's LP address (e.g., "2.2.2.2:41264")
    /// * `client_keypair` - Client's x25519 keypair
    /// * `gateway_lp_peer` - Encapsulates all the gateway keys needed for the Lewes Protocol
    /// * `ciphersuite` - the set of cryptographic protocols to use when negotiating the session with the node
    /// * `gateway_supported_lp_protocol_version` - Gateway's LP protocol version
    pub fn new(
        exit_address: SocketAddr,
        client_keypair: Arc<DHKeyPair>,
        gateway_lp_peer: LpRemotePeer,
        ciphersuite: Ciphersuite,
        gateway_supported_lp_protocol_version: u8,
    ) -> Self {
        let lp_local_peer = LpLocalPeer::new(ciphersuite, client_keypair);

        let lp_protocol = if gateway_supported_lp_protocol_version > version::CURRENT {
            warn!(
                "suggested LP protocol ({gateway_supported_lp_protocol_version}) is higher  than the current known version. attempting to downgrade it to {}",
                version::CURRENT
            );
            version::CURRENT
        } else {
            gateway_supported_lp_protocol_version
        };

        Self {
            exit_address,
            lp_local_peer,
            gateway_lp_peer,
            gateway_supported_lp_protocol_version: lp_protocol,
            transport_session: None,
        }
    }

    fn state_machine_mut(&mut self) -> Result<&mut LpTransportSession> {
        self.transport_session
            .as_mut()
            .ok_or(LpClientError::IncompleteHandshake)
    }

    /// The gateway this session is with, which is also where its frames have to be forwarded.
    pub fn exit_address(&self) -> SocketAddr {
        self.exit_address
    }

    /// Returns whether the handshake has completed and the session can carry frames.
    pub fn is_handshake_complete(&self) -> bool {
        self.transport_session.is_some()
    }

    /// Discard the session, so the next [`Self::perform_handshake`] starts from nothing.
    ///
    /// A handshake cannot be resumed halfway, so a retry has to begin from a clean slate.
    pub fn reset(&mut self) {
        self.transport_session = None;
    }

    /// Attempt to wrap the provided `LpFrame` into a `EncryptedLpPacket`
    /// using the inner state machine.
    pub fn prepare_transport_packet(&mut self, frame: LpFrame) -> Result<EncryptedLpPacket> {
        let state_machine = self.state_machine_mut()?;
        prepare_send_packet(frame, state_machine)
    }

    /// Attempt to recover received `LpFrame` from the received `EncryptedLpPacket`
    /// using the inner state machine.
    pub fn extract_forwarded_response(
        &mut self,
        response_packet: EncryptedLpPacket,
    ) -> Result<LpFrame> {
        let state_machine = self.state_machine_mut()?;
        extract_forwarded_response(response_packet, state_machine)
    }

    /// Performs the LP handshake with the exit gateway by forwarding packets
    /// through the entry gateway.
    ///
    /// This method:
    /// 1. Runs handshake loop, forwarding all packets through entry gateway
    /// 2. Stores established session in internal state machine
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
    pub async fn perform_handshake<S>(
        &mut self,
        outer_client: &mut LpGatewayClient<S>,
    ) -> Result<()>
    where
        S: LpTransportChannel + LpHandshakeChannel + Unpin,
    {
        if self.lp_local_peer.ciphersuite().kem() == KEM::McEliece {
            return Err(LpClientError::UnsupportedNestedMcEliece);
        }

        tracing::debug!(
            "Starting nested LP handshake with exit gateway {}",
            self.exit_address
        );

        let mut nested_connection = outer_client.as_nested_connection(self.exit_address);

        let local_peer = self.lp_local_peer.clone();
        let remote_peer = self.gateway_lp_peer.clone();
        let protocol_version = self.gateway_supported_lp_protocol_version;

        let session = LpTransportSession::psq_handshake_initiator(
            &mut nested_connection,
            local_peer,
            remote_peer,
            protocol_version,
            HandshakeMode::OneWayExit,
        )?
        .complete_handshake()
        .await?;

        // Store the state machine (with established session) for later use
        self.transport_session = Some(session);
        debug!("completed nested handshake");
        Ok(())
    }
}
