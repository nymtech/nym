// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::lp::cleanup::CleanupTask;
use crate::node::lp::control::egress::dialer::LpDialer;
use crate::node::lp::control::ingress::listener::LpControlListener;
use crate::node::lp::{SharedGatewayLpControlState, SharedLpControlState};

use nym_gateway::node::ClientRegistry;
use nym_task::ShutdownTracker;
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
    /// Both states are built outside, so what this node is was decided before any of this ran:
    /// `gateway_state` is `None` on a node that serves no clients.
    pub fn new(
        shared_state: SharedLpControlState,
        gateway_state: Option<SharedGatewayLpControlState>,
        clients: ClientRegistry,
        shutdown: ShutdownTracker,
    ) -> Result<Self, NymNodeError> {
        let lp_config = shared_state.lp_config;

        let dialer = LpDialer::new(shared_state.clone(), &lp_config.debug, shutdown.clone());

        let sessions = shared_state.sessions.clone();
        let registration_nonces = gateway_state
            .as_ref()
            .map(|state| state.registration_nonces.clone());

        let control_listener = LpControlListener::new(
            lp_config.control_bind_address,
            shared_state,
            gateway_state,
            shutdown.clone(),
        );
        let cleanup_task = CleanupTask::new(
            sessions,
            clients,
            registration_nonces,
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
