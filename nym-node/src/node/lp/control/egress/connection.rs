// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::active_sessions::LpPeer;
use crate::node::lp::control::stats::LpConnectionStats;
use crate::node::lp::directory::LpNodeDetails;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::SharedLpNodeControlState;
use nym_lp::LpTransportSession;
use nym_lp::transport::{LpHandshakeChannel, LpTransportChannel};
use nym_lp_data::packet::header::LpReceiverIndex;
use std::net::SocketAddr;
use tracing::debug;

/// Initial connection handler for an egress LP node before completing the KKT/PSQ handshake.
pub struct InitialLpEgressNodeConnectionHandler<S> {
    stream: S,
    remote_addr: SocketAddr,
    responder_details: LpNodeDetails,

    state: SharedLpNodeControlState,
    stats: LpConnectionStats,
}

impl<S> InitialLpEgressNodeConnectionHandler<S>
where
    S: LpHandshakeChannel + LpTransportChannel + Unpin,
{
    pub(crate) fn new(
        stream: S,
        remote_addr: SocketAddr,
        responder_details: LpNodeDetails,
        state: SharedLpNodeControlState,
    ) -> Self {
        Self {
            stream,
            remote_addr,
            responder_details,
            state,
            stats: LpConnectionStats::new(),
        }
    }

    /// Run the mutual KKT/PSQ handshake and store the resulting session.
    ///
    /// The TCP connection is only a handshake carrier: on success the session is deposited in
    /// the shared node-session map (demoting any previous session for this peer) and the
    /// connection is dropped. The session is subsequently used by the *data plane* over UDP.
    pub(crate) async fn complete_initial_handshake(
        mut self,
    ) -> Result<LpReceiverIndex, LpHandlerError> {
        let remote = self.remote_addr;

        if self.responder_details.kem_key_hashes.is_empty() {
            return Err(LpHandlerError::MissingNodeKEMKeyHashes {
                node_ip: self.remote_addr.ip(),
                node_id: self.responder_details.node_id,
            });
        }

        // 1. complete KKT/PSQ handshake before doing anything else.
        // bail if it takes too long
        let timeout = self.state.shared.lp_config.debug.handshake_ttl;
        let stream = &mut self.stream;

        let handshake_state = match LpTransportSession::psq_handshake_initiator_mutual_internode(
            stream,
            self.state.local_lp_peer.clone(),
            self.responder_details.to_lp_peer(),
            self.responder_details.supported_protocol,
        ) {
            Ok(handshake_state) => handshake_state,
            Err(err) => {
                debug!("failed to initiate mutual KTT/PSQ handshake with {remote}: {err}");
                self.stats.emit_lifecycle_node_metrics(false);
                return Err(err.into());
            }
        };

        let session = match tokio::time::timeout(timeout, handshake_state.complete_handshake())
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

        debug!(
            "completed egress KKT/PSQ handshake with node {}: {remote}",
            self.responder_details.node_id
        );

        let receiver_index = session.receiver_index();
        self.state
            .shared
            .sessions
            .insert_addressed_session(LpPeer::node(remote.ip()), session)?;

        debug!(
            "stored LP session with node {} ({}); closing control connection",
            self.responder_details.node_id,
            remote.ip()
        );

        Ok(receiver_index)
    }
}
