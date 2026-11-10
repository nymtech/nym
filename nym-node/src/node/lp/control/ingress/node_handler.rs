// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::active_sessions::LpPeer;
use crate::node::lp::control::stats::LpConnectionStats;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::SharedLpControlState;
use nym_lp::LpTransportSession;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::debug;

/// Handler for an LP control connection from another node, once its session is established.
pub(crate) struct LpNodeConnectionHandler<S = TcpStream> {
    // nothing reads this until the control connection carries node traffic
    #[allow(dead_code)]
    stream: S,
    remote_addr: SocketAddr,

    // nothing reads this until the control connection carries node traffic
    #[allow(dead_code)]
    state: SharedLpControlState,
    stats: LpConnectionStats,
}

impl<S> LpNodeConnectionHandler<S> {
    /// Deposit `session` as the node's at `remote_addr`, continuing the connection's `stats`.
    pub(crate) fn with_session(
        stream: S,
        remote_addr: SocketAddr,
        state: SharedLpControlState,
        session: LpTransportSession,
        stats: LpConnectionStats,
    ) -> Result<Self, LpHandlerError> {
        state
            .sessions
            .insert_addressed_session(LpPeer::node(remote_addr.ip()), session)?;

        Ok(Self {
            stream,
            remote_addr,
            state,
            stats,
        })
    }

    /// Nothing is exchanged on a node's control connection yet; it ends once the session is stored.
    pub(crate) async fn run(&mut self) -> Result<(), LpHandlerError> {
        debug!(
            "stored LP session with node at {}; closing control connection",
            self.remote_addr.ip()
        );

        self.stats.emit_lifecycle_node_metrics(true);
        Ok(())
    }
}
