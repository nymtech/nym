// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::control::LpConnectionStats;
use crate::node::lp::directory::LpNodeDetails;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::SharedLpNodeControlState;
use nym_lp::LpTransportSession;
use nym_lp::transport::{LpHandshakeChannel, LpTransportChannel};
use nym_metrics::inc;
use nym_node_metrics::NymNodeMetrics;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::debug;

/// Initial connection handler for an LP node before completing the KKT/PSQ handshake.
pub struct InitialLpNodeConnectionHandler<S = TcpStream> {
    stream: S,
    remote_addr: SocketAddr,
    initiator_details: LpNodeDetails,

    state: SharedLpNodeControlState,
    stats: LpConnectionStats,
}

impl<S> InitialLpNodeConnectionHandler<S>
where
    S: LpHandshakeChannel + Unpin,
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
            stats: Default::default(),
        }
    }

    pub(crate) fn metrics(&self) -> &NymNodeMetrics {
        &self.state.shared.metrics
    }

    pub async fn handle(mut self) -> Result<(), LpHandlerError> {
        // Track total LP connections handled
        inc!("lp_node_connections_total");
        let remote = self.remote_addr;

        if self.initiator_details.kem_key_hashes.is_empty() {
            return Err(LpHandlerError::MissingNodeKEMKeyHashes {
                node_ip: self.remote_addr.ip(),
                node_id: self.initiator_details.node_id,
            });
        }

        // 1. complete KKT/PSQ handshake before doing anything else.
        // bail if it takes too long
        let timeout = self.state.shared.lp_config.debug.handshake_ttl;
        let local_peer = self.state.local_lp_peer.clone();
        let stream = &mut self.stream;

        let session = match tokio::time::timeout(timeout, async move {
            LpTransportSession::psq_handshake_responder_mutual(
                stream,
                local_peer,
                self.initiator_details.kem_key_hashes,
            )
            .complete_handshake()
            .await
        })
        .await
        {
            Err(_timeout) => {
                debug!("timed out attempting to complete mutual KTT/PSQ handshake with {remote}");
                self.stats.emit_lifecycle_node_metrics(false);
                return Ok(());
            }
            Ok(Err(handshake_failure)) => {
                debug!(
                    "failed to complete mutual KKT/PSQ handshake with {remote}: {handshake_failure}"
                );
                self.stats.emit_lifecycle_node_metrics(false);
                return Ok(());
            }
            Ok(Ok(session)) => session,
        };

        LpNodeConnectionHandler {
            stream,
            remote_addr: remote,
            state: self.state,
            stats: self.stats,
            transport_session: session,
        }
        .handle()
        .await
    }
}

/// Connection handler for an LP node after completing the KKT/PSQ handshake.
pub struct LpNodeConnectionHandler<S = TcpStream> {
    stream: S,
    remote_addr: SocketAddr,

    state: SharedLpNodeControlState,
    stats: LpConnectionStats,
    transport_session: LpTransportSession,
}

impl<S> LpNodeConnectionHandler<S>
where
    S: LpTransportChannel + Unpin,
{
    async fn handle(mut self) -> Result<(), LpHandlerError> {
        todo!();

        self.stats.emit_lifecycle_node_metrics(true);
        Ok(())
    }
}
