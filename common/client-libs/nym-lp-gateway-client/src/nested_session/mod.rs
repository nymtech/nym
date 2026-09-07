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
use nym_lp::peer::{DHKeyPair, LpLocalPeer, LpRemotePeer};
use nym_lp::psq::initiator::HandshakeMode;
use nym_lp::transport::LpHandshakeChannel;
use nym_lp::transport::traits::{LpDatagramChannel, LpTransportChannel};
use nym_lp::{Ciphersuite, KEM, LpTransportSession};
use nym_lp_data::packet::version;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, warn};

pub mod connection;

/// What it takes to handshake with an exit gateway by forwarding through an entry one.
///
/// Has no connection of its own: an established [`LpGatewayClient`] carries its packets, so that
/// channel - and the outer session that encrypts the forwarding envelope - are passed in to
/// [`Self::perform_handshake`], which hands the inner session back.
///
/// # Example
///
/// ```ignore
/// // Outer session already established with the entry gateway
/// let mut client = LpGatewayClient::new(config);
/// let mut outer = client.handshake(entry, ...).await?;
///
/// // Now establish the inner session with the exit gateway
/// let inner = NestedLpSession::new(exit_address, client_keypair, exit_peer, ciphersuite, exit_lp_protocol)
///     .perform_handshake(&mut client, entry, &mut outer)
///     .await?;
///
/// // ... send frames on `inner`, forwarded by the entry gateway ...
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
        Self {
            exit_address,
            lp_local_peer: LpLocalPeer::new(ciphersuite, client_keypair),
            gateway_lp_peer,
            gateway_supported_lp_protocol_version,
        }
    }

    /// The gateway this session is with, which is also where its frames have to be forwarded.
    pub fn exit_address(&self) -> SocketAddr {
        self.exit_address
    }

    /// Handshake with the exit gateway, forwarding every packet through the entry gateway, and
    /// hand the resulting session over.
    ///
    /// # Arguments
    /// * `outer_client` - the transport holding a control connection to the entry gateway
    /// * `outer_gateway` - the entry gateway doing the forwarding
    /// * `outer_session` - the established session with that entry gateway
    ///
    /// # Errors
    /// Returns an error if:
    /// - Packet serialization/parsing fails
    /// - Forwarding through entry gateway fails
    /// - Exit gateway handshake fails
    /// - Cryptographic operations fail
    pub async fn perform_handshake<S, D>(
        &self,
        outer_client: &mut LpGatewayClient<S, D>,
        outer_gateway: SocketAddr,
        outer_session: &mut LpTransportSession,
    ) -> Result<LpTransportSession>
    where
        S: LpTransportChannel + LpHandshakeChannel + Unpin,
        D: LpDatagramChannel,
    {
        if self.lp_local_peer.ciphersuite().kem() == KEM::McEliece {
            return Err(LpClientError::UnsupportedNestedMcEliece);
        }

        let advertised = self.gateway_supported_lp_protocol_version;
        let version = version::negotiate(advertised)
            .ok_or(LpClientError::UnsupportedProtocolVersion { advertised })?;

        if version != advertised {
            warn!(
                "exit gateway {} suggested LP protocol {advertised}; speaking {version} instead",
                self.exit_address
            );
        }

        tracing::debug!(
            "Starting nested LP handshake with exit gateway {}",
            self.exit_address
        );

        let mut nested_connection =
            outer_client.as_nested_connection(outer_gateway, self.exit_address, outer_session);

        let session = LpTransportSession::psq_handshake_initiator(
            &mut nested_connection,
            self.lp_local_peer.clone(),
            self.gateway_lp_peer.clone(),
            version,
            HandshakeMode::OneWayExit,
        )?
        .complete_handshake()
        .await?;

        debug!("completed nested handshake");
        Ok(session)
    }
}
