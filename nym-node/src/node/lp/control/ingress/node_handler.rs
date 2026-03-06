// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::lp::control::LpConnectionStats;
use crate::node::lp::directory::LpNodeDetails;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::SharedLpNodeControlState;
use nym_lp::LpTransportSession;
use nym_lp::transport::{LpHandshakeChannel, LpTransportChannel};
use nym_metrics::inc;
use nym_node_metrics::NymNodeMetrics;
use nym_topology::NodeId;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::debug;

/// Initial connection handler for an ingress LP node before completing the KKT/PSQ handshake.
pub struct InitialLpIngressNodeConnectionHandler<S = TcpStream> {
    stream: S,
    remote_addr: SocketAddr,
    initiator_details: LpNodeDetails,

    state: SharedLpNodeControlState,
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
        state: SharedLpNodeControlState,
    ) -> Self {
        Self {
            stream,
            remote_addr,
            initiator_details,
            state,
            stats: LpConnectionStats::new(),
        }
    }

    pub(crate) fn metrics(&self) -> &NymNodeMetrics {
        &self.state.shared.metrics
    }

    pub(crate) async fn complete_initial_handshake(
        mut self,
    ) -> Option<Result<LpIngressNodeConnectionHandler<S>, LpHandlerError>> {
        let remote = self.remote_addr;

        if self.initiator_details.kem_key_hashes.is_empty() {
            return Some(Err(LpHandlerError::MissingNodeKEMKeyHashes {
                node_ip: self.remote_addr.ip(),
                node_id: self.initiator_details.node_id,
            }));
        }

        // 1. complete KKT/PSQ handshake before doing anything else.
        // bail if it takes too long
        let timeout = self.state.shared.lp_config.debug.handshake_ttl;
        let local_peer = self.state.local_lp_peer.clone();
        let stream = &mut self.stream;
        let kem_hashes = self.initiator_details.kem_key_hashes.clone();

        let session = match tokio::time::timeout(timeout, async move {
            LpTransportSession::psq_handshake_responder_mutual(stream, local_peer, kem_hashes)
                .complete_handshake()
                .await
        })
        .await
        {
            Err(_timeout) => {
                debug!("timed out attempting to complete mutual KTT/PSQ handshake with {remote}");
                self.stats.emit_lifecycle_node_metrics(false);
                return None;
            }
            Ok(Err(handshake_failure)) => {
                debug!(
                    "failed to complete mutual KKT/PSQ handshake with {remote}: {handshake_failure}"
                );
                self.stats.emit_lifecycle_node_metrics(false);
                return None;
            }
            Ok(Ok(session)) => session,
        };

        debug!(
            "completed ingress KKT/PSQ handshake with node {}: {remote}",
            self.initiator_details.node_id
        );

        Some(Ok(LpIngressNodeConnectionHandler {
            stream: self.stream,
            remote_addr: remote,
            remote_node_id: self.initiator_details.node_id,
            state: self.state,
            stats: self.stats,
            transport_session: session,
        }))
    }

    pub async fn handle(mut self) -> Result<(), LpHandlerError> {
        // Track total LP connections handled
        inc!("lp_node_connections_total");

        // attempt to complete initial handshake
        let upgraded_handler = match self.complete_initial_handshake().await {
            None => return Ok(()),
            Some(handler_res) => handler_res?,
        };

        // continue handling the requests with the transport session
        upgraded_handler.handle().await
    }
}

/// Connection handler for an LP node after completing the KKT/PSQ handshake.
pub struct LpIngressNodeConnectionHandler<S = TcpStream> {
    stream: S,
    remote_addr: SocketAddr,
    remote_node_id: NodeId,

    state: SharedLpNodeControlState,
    stats: LpConnectionStats,
    transport_session: LpTransportSession,
    // LOCAL receiver index to stream id
    // client_streams: HashMap<ReceiverIndex, ClientStreamId>,
}

impl<S> LpIngressNodeConnectionHandler<S>
where
    S: LpHandshakeChannel + LpTransportChannel + Unpin,
{
    async fn handle(mut self) -> Result<(), LpHandlerError> {
        // handle all the forwarding here

        self.stats.emit_lifecycle_node_metrics(true);
        Ok(())
    }

    pub(crate) fn transport_session(&self) -> &LpTransportSession {
        &self.transport_session
    }
}
