// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpDebug;
use crate::node::lp::active_sessions::{ActiveLpSessions, LpPeer};
use nym_gateway::node::ClientRegistry;
use nym_metrics::inc_by;
use std::time::Instant;
use tracing::{debug, info};

/// What a [`ActiveLpSessions::remove_stale`] pass took out.
#[derive(Default)]
pub(crate) struct Eviction {
    pub(crate) live_removed: u64,
    pub(crate) demoted_removed: u64,

    /// Peers left with no session at all, so a caller holding state keyed by peer can drop it too.
    pub(crate) forgotten_peers: Vec<LpPeer>,
}

pub(crate) struct CleanupTask {
    /// Every established LP session, clients and nodes alike.
    sessions: ActiveLpSessions,

    /// Where clients were last seen. Swept alongside the sessions, since an address is only
    /// meaningful while the client it belongs to still has one.
    clients: ClientRegistry,

    cfg: LpDebug,
    shutdown: nym_task::ShutdownToken,
}

impl CleanupTask {
    pub fn new(
        sessions: ActiveLpSessions,
        clients: ClientRegistry,
        cfg: LpDebug,
        shutdown: nym_task::ShutdownToken,
    ) -> Self {
        CleanupTask {
            sessions,
            clients,
            cfg,
            shutdown,
        }
    }

    fn perform_cleanup(&self) {
        let start = Instant::now();

        let eviction = self.sessions.remove_stale(
            self.cfg.session_ttl,
            self.cfg.internode_session_ttl,
            self.cfg.read_only_session_ttl,
        );

        // a client with no session left cannot be addressed, so its last-seen address is dead
        // weight; nothing else ever removes one
        for peer in &eviction.forgotten_peers {
            if let LpPeer::Client(client) = peer {
                self.clients.forget(*client);
            }
        }

        let live_removed = eviction.live_removed;
        let demoted_removed = eviction.demoted_removed;

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
