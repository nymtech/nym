// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::active_sessions::LpPeer;
use crate::node::lp::control::ingress::client_handler::LpClientConnectionHandler;
use crate::node::lp::control::stats::LpConnectionStats;
use crate::node::lp::directory::LpNodeDetails;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::{SharedGatewayLpControlState, SharedLpControlState};
use nym_lp::LpTransportSession;
use nym_lp::transport::{LpHandshakeChannel, LpTransportChannel};
use nym_metrics::inc;
use nym_node_metrics::NymNodeMetrics;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::{debug, warn};

/// Initial connection handler for an ingress LP node before completing the KKT/PSQ handshake.
pub struct InitialLpIngressNodeConnectionHandler<S = TcpStream> {
    stream: S,
    remote_addr: SocketAddr,
    initiator_details: LpNodeDetails,

    state: SharedLpControlState,

    /// What a forwarded client is served with; absent on a node that forwards no client traffic.
    gateway_state: Option<SharedGatewayLpControlState>,
    stats: LpConnectionStats,
}

impl<S> InitialLpIngressNodeConnectionHandler<S>
where
    S: LpHandshakeChannel + LpTransportChannel + Unpin,
{
    pub fn new(
        stream: S,
        remote_addr: SocketAddr,
        initiator_details: LpNodeDetails,
        state: SharedLpControlState,
        gateway_state: Option<SharedGatewayLpControlState>,
    ) -> Self {
        Self {
            stream,
            remote_addr,
            initiator_details,
            state,
            gateway_state,
            stats: LpConnectionStats::new(),
        }
    }

    pub(crate) fn metrics(&self) -> &NymNodeMetrics {
        &self.state.metrics
    }

    async fn complete_initial_handshake(&mut self) -> Result<LpTransportSession, LpHandlerError> {
        let remote = self.remote_addr;

        if self.initiator_details.kem_key_hashes.is_empty() {
            return Err(LpHandlerError::MissingNodeKEMKeyHashes {
                node_ip: self.remote_addr.ip(),
                node_id: self.initiator_details.node_id,
            });
        }

        // 1. complete KKT/PSQ handshake before doing anything else.
        // bail if it takes too long
        let timeout = self.state.lp_config.debug.handshake_ttl;
        let local_peer = self.state.local_lp_peer.clone();
        let stream = &mut self.stream;
        let kem_hashes = self.initiator_details.kem_key_hashes.clone();

        let session = match tokio::time::timeout(timeout, async move {
            LpTransportSession::psq_handshake_responder(stream, local_peer, Some(kem_hashes))
                .complete_handshake()
                .await
        })
        .await
        {
            Err(_timeout) => {
                debug!("timed out attempting to complete mutual KTT/PSQ handshake with {remote}");
                self.stats.emit_lifecycle_node_metrics(false);
                return Err(LpHandlerError::HandshakeTimeout);
            }
            Ok(Err(handshake_failure)) => {
                debug!(
                    "failed to complete mutual KKT/PSQ handshake with {remote}: {handshake_failure}"
                );
                self.stats.emit_lifecycle_node_metrics(false);
                return Err(handshake_failure.into());
            }
            Ok(Ok(session)) => session,
        };

        debug!("completed ingress KKT/PSQ handshake with {remote}");
        Ok(session)
    }

    fn store_node_session(&self, session: LpTransportSession) -> Result<(), LpHandlerError> {
        self.state
            .sessions
            .insert_addressed_session(LpPeer::node(self.remote_addr.ip()), session)?;

        debug!(
            "stored LP session with node {} ({}); closing control connection",
            self.initiator_details.node_id,
            self.remote_addr.ip()
        );

        self.stats.emit_lifecycle_node_metrics(true);
        Ok(())
    }

    /// Serve the connection as the client handler would, since the opener was not a node's.
    async fn serve_forwarded_client(
        self,
        session: LpTransportSession,
    ) -> Result<(), LpHandlerError> {
        let remote = self.remote_addr;

        let Some(gateway_state) = self.gateway_state else {
            debug!(
                "refusing forwarded LP client connection from {remote}: this node forwards no client traffic"
            );
            inc!("lp_client_connections_refused");
            return Ok(());
        };

        let active_connections = self
            .state
            .metrics
            .network
            .active_lp_client_connections_count();
        let max_connections = self.state.lp_config.debug.max_connections;
        if active_connections >= max_connections {
            warn!(
                "LP connection limit exceeded ({active_connections}/{max_connections}), rejecting forwarded connection from {remote}"
            );
            return Ok(());
        }

        let mut handler = LpClientConnectionHandler::with_session(
            self.stream,
            remote,
            self.state,
            gateway_state,
            session,
        )?;

        inc!("lp_client_connections_total");
        inc!("lp_forwarded_client_connections_total");
        let metrics = handler.metrics().clone();
        metrics.network.new_ingress_lp_client_connection();
        let result = handler.run().await;
        metrics.network.closed_ingress_lp_client_connection();
        result
    }

    pub async fn handle(mut self) -> Result<(), LpHandlerError> {
        // Track total LP connections handled
        inc!("lp_node_connections_total");

        let session = self.complete_initial_handshake().await?;

        // the KKT step yields the initiator's key only for a mutual opener whose key matched the
        // directory, so its presence is what makes this a node's session rather than a client's
        if session.session_binding().initiator_pq_pk.is_some() {
            self.store_node_session(session)
        } else {
            self.serve_forwarded_client(session).await
        }
    }
}
