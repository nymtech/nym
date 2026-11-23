// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::control::ingress::client_handler::LpClientConnectionHandler;
use crate::node::lp::control::ingress::handshake::complete_ingress_handshake;
use crate::node::lp::control::ingress::node_handler::LpNodeConnectionHandler;
use crate::node::lp::control::stats::LpConnectionStats;
use crate::node::lp::directory::LpNodeDetails;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::{SharedGatewayLpControlState, SharedLpControlState};
use nym_lp::LpTransportSession;
use nym_lp::session::PersistentSessionBinding;
use nym_lp::transport::{LpHandshakeChannel, LpTransportChannel};
use nym_metrics::inc;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::{debug, warn};

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

    /// Before the handshake, the address is all a connection can be accounted by.
    fn is_from_node_address(&self) -> bool {
        self.known_node.is_some()
    }

    /// What a client on this connection is served with, logging why if there is nothing.
    fn admit_client(&self) -> Option<&SharedGatewayLpControlState> {
        let remote = self.remote_addr;

        // Without gateway state there is nothing this node could do for them: serving the
        // connection would mean completing a handshake and accepting a registration only to drop
        // everything that followed. Refusing is what makes that visible to the caller instead of
        // looking like a dead network.
        let Some(gateway_state) = &self.gateway_state else {
            debug!(
                "refusing LP client connection from {remote}: this node forwards no client traffic"
            );
            inc!("lp_client_connections_refused");
            return None;
        };

        let active = self
            .state
            .metrics
            .network
            .active_lp_client_connections_count();
        let max = self.state.lp_config.debug.max_connections;
        if active >= max {
            warn!(
                "LP connection limit exceeded ({active}/{max}), rejecting connection from {remote}"
            );
            return None;
        }

        Some(gateway_state)
    }

    pub async fn handle(self) -> Result<(), LpHandlerError> {
        let metrics = self.state.metrics.clone();
        let from_node_address = self.is_from_node_address();

        // only a plain client can be turned away before the handshake, and it is checked before it
        // counts or it would count against the limit itself; a node's address is always taken
        // through the handshake, since only its opener tells the node apart from a client it
        // forwards
        if !from_node_address && self.admit_client().is_none() {
            return Ok(());
        }

        // the limit reads the client gauge, so a plain client counts from here on; a node's
        // address counts on the node gauge until its handler ends, as it did before
        if from_node_address {
            metrics.network.new_ingress_lp_node_connection();
        } else {
            metrics.network.new_ingress_lp_client_connection();
        }
        let result = self.handle_connection().await;
        if from_node_address {
            metrics.network.closed_ingress_lp_node_connection();
        } else {
            metrics.network.closed_ingress_lp_client_connection();
        }
        result
    }

    async fn handle_connection(mut self) -> Result<(), LpHandlerError> {
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
        }
        let outcome =
            complete_ingress_handshake(&mut self.stream, &self.state, initiator_kem_hashes).await;

        let session = match outcome {
            Ok(session) => session,
            Err(err) => return self.handshake_failed(err),
        };
        debug!("completed ingress KKT/PSQ handshake with {remote}");

        // a one-way opener is a client's; a mutual one claims to be the node listed here
        if session.session_binding().initiator_pq_pk.is_none() {
            self.serve_client(session).await
        } else {
            self.serve_node(session).await
        }
    }

    /// Whether a mutual opener really is the listed node's, by the x25519 key the handshake proved.
    fn check_node_identity(
        &self,
        binding: &PersistentSessionBinding,
    ) -> Result<(), LpHandlerError> {
        let node_ip = self.remote_addr.ip();

        let Some(node) = &self.known_node else {
            return Err(LpHandlerError::NotLpNode { ip_addr: node_ip });
        };

        // the KEM key the opener carried only shows it knew that node's public key, which every
        // client learns from its own KKT response with the node; the x25519 key the PSQ handshake
        // proved possession of is what the directory lists for the node as well
        let proven_key = binding.initiator_x25519();
        if !proven_key.is_some_and(|key| key.as_ref() == node.x25519.as_ref()) {
            return Err(LpHandlerError::MismatchedNodeIdentity {
                node_ip,
                node_id: node.node_id,
            });
        }

        Ok(())
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

    async fn serve_client(self, session: LpTransportSession) -> Result<(), LpHandlerError> {
        let remote = self.remote_addr;
        let forwarded = self.is_from_node_address();

        // a plain client was admitted at the door, before its handshake; a forwarded one is
        // admitted only now, and counted as a client from here
        let gateway_state = if forwarded {
            self.admit_client()
        } else {
            self.gateway_state.as_ref()
        };
        let Some(gateway_state) = gateway_state.cloned() else {
            return Ok(());
        };

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
            metrics.network.new_ingress_lp_client_connection();
        }
        let result = handler.run().await;
        if forwarded {
            metrics.network.closed_ingress_lp_client_connection();
        }
        result
    }

    async fn serve_node(self, session: LpTransportSession) -> Result<(), LpHandlerError> {
        if let Err(err) = self.check_node_identity(session.session_binding()) {
            // neither stored nor served
            self.stats.emit_lifecycle_node_metrics(false);
            return Err(err);
        }

        let mut handler = LpNodeConnectionHandler::with_session(
            self.stream,
            self.remote_addr,
            self.state,
            session,
            self.stats,
        )?;
        handler.run().await
    }
}
