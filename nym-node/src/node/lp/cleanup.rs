// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpDebug;
use crate::node::lp::active_sessions::ActiveLpSessions;
use nym_metrics::inc_by;
use std::time::Instant;
use tracing::{debug, info};

pub(crate) struct CleanupTask {
    /// Client-facing sessions.
    client_session_states: ActiveLpSessions,

    /// Node-to-node sessions. Separate key space and different behavior
    node_session_states: ActiveLpSessions,

    cfg: LpDebug,
    shutdown: nym_task::ShutdownToken,
}

impl CleanupTask {
    pub fn new(
        client_session_states: ActiveLpSessions,
        node_session_states: ActiveLpSessions,
        cfg: LpDebug,
        shutdown: nym_task::ShutdownToken,
    ) -> Self {
        CleanupTask {
            client_session_states,
            node_session_states,
            cfg,
            shutdown,
        }
    }

    fn perform_cleanup(&self) {
        let start = Instant::now();
        let demoted_ttl = self.cfg.read_only_session_ttl;

        let (client_removed, client_demoted) = self
            .client_session_states
            .remove_stale(self.cfg.session_ttl, demoted_ttl);

        let (node_removed, node_demoted) = self
            .node_session_states
            .remove_stale(self.cfg.internode_session_ttl, demoted_ttl);

        let live_removed = client_removed + node_removed;
        let demoted_removed = client_demoted + node_demoted;

        if live_removed > 0 || demoted_removed > 0 {
            let duration = start.elapsed();
            info!(
                "LP state cleanup: {live_removed} sessions, {demoted_removed} demoted (took {:.3}s)",
                duration.as_secs_f64()
            );

            if live_removed > 0 {
                inc_by!("lp_states_cleanup_session_removed", live_removed as i64);
            }
            if demoted_removed > 0 {
                inc_by!("lp_states_cleanup_demoted_removed", demoted_removed as i64);
            }
        }
    }

    /// Background loop for cleaning up stale state entries.
    ///
    /// Scans the client and node session maps, removing entries idle beyond their TTL.
    /// Sessions in [`LpSessionState::ReadOnlyTransport`] use a shorter TTL, since they only
    /// have to outlive packets already in flight towards them after being superseded.
    ///
    pub(crate) async fn run(&self) {
        let interval = self.cfg.state_cleanup_interval;

        let mut cleanup_interval = tokio::time::interval(interval);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    debug!("LP state cleanup task: received shutdown signal");
                    break;
                }
                _ = cleanup_interval.tick() => {
                    self.perform_cleanup();
                }
            }
        }

        info!("LP state cleanup task shutdown complete");
    }
}
