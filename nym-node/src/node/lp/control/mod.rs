// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpConfig;
use crate::error::NymNodeError;
use crate::node::lp::active_sessions::ActiveLpSessions;
use crate::node::lp::cleanup::CleanupTask;
use crate::node::lp::control::egress::dialer::LpDialer;
use crate::node::lp::control::ingress::listener::LpControlListener;
use crate::node::lp::directory::LpNodes;
use crate::node::lp::state::SharedLpNodeControlState;
use crate::node::lp::{SharedLpClientControlState, SharedLpState};

use nym_gateway::node::wireguard::PeerRegistrator;
use nym_lp::peer::LpLocalPeer;
use nym_node_metrics::NymNodeMetrics;
use nym_task::ShutdownTracker;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::error;

mod control_tests;
pub mod egress;
pub mod ingress;
mod stats;

pub struct LpControlSetup {
    /// Listens for incoming connections
    control_listener: LpControlListener,

    /// Establishes node-to-node sessions on demand.
    dialer: LpDialer,

    // Cleans up stale sessions
    cleanup_task: CleanupTask,

    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl LpControlSetup {
    #[expect(clippy::too_many_arguments)]
    pub async fn new(
        local_lp_peer: LpLocalPeer,
        lp_config: LpConfig,
        metrics: NymNodeMetrics,
        peer_registrator: Option<PeerRegistrator>,
        network_nodes: LpNodes,
        client_sessions: ActiveLpSessions,
        node_sessions: ActiveLpSessions,
        shutdown: ShutdownTracker,
    ) -> Result<Self, NymNodeError> {
        let shared_lp_state = SharedLpState { metrics, lp_config };

        let client_control_state = SharedLpClientControlState {
            local_lp_peer: local_lp_peer.clone(),
            peer_registrator,
            forward_semaphore: Arc::new(Semaphore::new(lp_config.debug.max_concurrent_forwards)),
            session_states: client_sessions.clone(),
            shared: shared_lp_state.clone(),
        };

        let nodes_control_state = SharedLpNodeControlState {
            local_lp_peer,
            nodes: network_nodes,
            node_sessions: node_sessions.clone(),
            shared: shared_lp_state.clone(),
        };

        let dialer = LpDialer::new(
            nodes_control_state.clone(),
            &lp_config.debug,
            shutdown.clone_shutdown_token(),
        );

        let control_listener = LpControlListener::new(
            lp_config.control_bind_address,
            client_control_state,
            nodes_control_state,
            shutdown.clone(),
        );
        let cleanup_task = CleanupTask::new(
            client_sessions,
            node_sessions,
            lp_config.debug,
            shutdown.clone_shutdown_token(),
        );

        Ok(LpControlSetup {
            control_listener,
            cleanup_task,
            dialer,
            shutdown,
        })
    }

    /// Handle for asking for a node-to-node session to be established.
    pub fn dialer(&self) -> LpDialer {
        self.dialer.clone()
    }

    pub fn start_tasks(mut self) {
        // control listener
        let shutdown_token = self.shutdown.clone_shutdown_token();
        self.shutdown.try_spawn_named(
            async move {
                if let Err(err) = self.control_listener.run().await {
                    shutdown_token.cancel();
                    error!("LP control listener error: {err}");
                }
            },
            "LP::LpControlListener",
        );

        // cleanup task
        self.shutdown.try_spawn_named(
            async move { self.cleanup_task.run().await },
            "LP::CleanupTask",
        );

        // the dialer needs no task of its own: it spawns one per dial, on demand
    }
}
