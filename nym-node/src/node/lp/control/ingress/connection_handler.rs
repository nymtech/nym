// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::active_sessions::LpPeer;
use crate::node::lp::control::ingress::client_handler::LpClientConnectionHandler;
use crate::node::lp::control::ingress::handshake::complete_ingress_handshake;
use crate::node::lp::control::stats::LpConnectionStats;
use crate::node::lp::directory::LpNodeDetails;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::{SharedGatewayLpControlState, SharedLpControlState};
use nym_lp::LpTransportSession;
use nym_lp::transport::{LpHandshakeChannel, LpTransportChannel};
use nym_metrics::inc;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::debug;

/// Handler for an accepted LP control connection: one handshake, then whatever the opener was.
pub struct LpIngressConnectionHandler<S = TcpStream> {
    stream: S,
    remote_addr: SocketAddr,

    /// The node the directory lists at this address, whose digests verify a mutual opener.
    known_node: Option<LpNodeDetails>,

    state: SharedLpControlState,

    /// What a client is served with; absent on a node that forwards no client traffic.
    gateway_state: Option<SharedGatewayLpControlState>,
    stats: LpConnectionStats,
}

impl<S> LpIngressConnectionHandler<S>
where
    S: LpHandshakeChannel + LpTransportChannel + Unpin,
{
    pub fn new(
        stream: S,
        remote_addr: SocketAddr,
        known_node: Option<LpNodeDetails>,
        state: SharedLpControlState,
        gateway_state: Option<SharedGatewayLpControlState>,
    ) -> Self {
        Self {
            stream,
            remote_addr,
            known_node,
            state,
            gateway_state,
            stats: LpConnectionStats::new(),
        }
    }

    /// Until the handshake completes, the address is all there is to account a connection by:
    /// only the opener tells a node apart from a client it forwards.
    fn is_from_node_address(&self) -> bool {
        self.known_node.is_some()
    }

    pub async fn handle(mut self) -> Result<(), LpHandlerError> {
        let remote = self.remote_addr;
        debug!(
            "Handling LP connection from {remote} (from a node's address: {})",
            self.is_from_node_address()
        );

        let initiator_kem_hashes = match &self.known_node {
            // we must always do mutual KKT for another nym-node
            Some(node) if node.kem_key_hashes.is_empty() => {
                return Err(LpHandlerError::MissingNodeKEMKeyHashes {
                    node_ip: remote.ip(),
                    node_id: node.node_id,
                });
            }
            Some(node) => Some(node.kem_key_hashes.clone()),
            None => None,
        };

        if self.is_from_node_address() {
            inc!("lp_node_connections_total");
            self.state.metrics.network.new_ingress_lp_node_connection();
        }
        let outcome =
            complete_ingress_handshake(&mut self.stream, &self.state, initiator_kem_hashes).await;
        if self.is_from_node_address() {
            // for all intents and purposes after handshake and establishing session,
            // node connection is closed
            self.state
                .metrics
                .network
                .closed_ingress_lp_node_connection();
        }

        let session = match outcome {
            Ok(session) => session,
            Err(err) => return self.handshake_failed(err),
        };
        debug!("completed ingress KKT/PSQ handshake with {remote}");

        // the KKT step yields the initiator's key only for a mutual opener whose key matched the
        // directory, so its presence is what makes this a node's session rather than a client's
        if session.session_binding().initiator_pq_pk.is_some() {
            self.store_node_session(session)
        } else {
            self.serve_client(session).await
        }
    }

    /// A failed handshake is worth reporting from a node's address, and routine from anywhere else.
    fn handshake_failed(&self, err: LpHandlerError) -> Result<(), LpHandlerError> {
        debug!(
            "failed to complete KKT/PSQ handshake with {}: {err}",
            self.remote_addr
        );
        if self.is_from_node_address() {
            self.stats.emit_lifecycle_node_metrics(false);
            Err(err)
        } else {
            self.stats.emit_lifecycle_client_metrics(false);
            Ok(())
        }
    }

    fn store_node_session(&self, session: LpTransportSession) -> Result<(), LpHandlerError> {
        self.state
            .sessions
            .insert_addressed_session(LpPeer::node(self.remote_addr.ip()), session)?;

        debug!(
            "stored LP session with node at {}; closing control connection",
            self.remote_addr.ip()
        );

        self.stats.emit_lifecycle_node_metrics(true);
        Ok(())
    }

    async fn serve_client(self, session: LpTransportSession) -> Result<(), LpHandlerError> {
        let remote = self.remote_addr;
        let forwarded = self.is_from_node_address();

        let Some(gateway_state) = self.state.admit_client(remote, self.gateway_state.as_ref())
        else {
            return Ok(());
        };
        let gateway_state = gateway_state.clone();

        let metrics = self.state.metrics.clone();
        let mut handler = LpClientConnectionHandler::with_session(
            self.stream,
            remote,
            self.state,
            gateway_state,
            session,
            self.stats,
        )?;

        inc!("lp_client_connections_total");
        if forwarded {
            inc!("lp_forwarded_client_connections_total");
        }
        metrics.network.new_ingress_lp_client_connection();
        let result = handler.run().await;
        metrics.network.closed_ingress_lp_client_connection();
        result
    }
}
