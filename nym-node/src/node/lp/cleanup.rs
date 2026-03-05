// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpDebug;
use crate::node::lp::state::ActiveLpSessions;
use dashmap::DashMap;
use nym_lp::LpTransportSession;
use nym_lp::peer_config::LpReceiverIndex;
use nym_metrics::inc_by;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Wrapper for state entries with timestamp tracking for cleanup
///
/// This wrapper adds `created_at` and `last_activity` timestamps to state entries,
/// enabling TTL-based cleanup of stale handshakes and sessions.
pub struct TimestampedState<T> {
    /// The actual state (LpStateMachine or LpSession)
    pub state: T,

    /// When this state was created (never changes)
    created_at: std::time::Instant,

    /// Last activity timestamp (unix seconds, atomically updated)
    ///
    /// For handshakes: never updated (use created_at for TTL)
    /// For sessions: updated on every packet received
    last_activity: std::sync::atomic::AtomicU64,
}

impl<T> TimestampedState<T> {
    /// Create a new timestamped state
    pub fn new(state: T) -> Self {
        let now_instant = std::time::Instant::now();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            state,
            created_at: now_instant,
            last_activity: std::sync::atomic::AtomicU64::new(now_unix),
        }
    }

    /// Update last_activity timestamp (cheap, lock-free operation)
    pub fn touch(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_activity
            .store(now, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get age since creation
    #[allow(dead_code)]
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get time since last activity
    pub fn since_activity(&self) -> Duration {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = self
            .last_activity
            .load(std::sync::atomic::Ordering::Relaxed);
        Duration::from_secs(now.saturating_sub(last))
    }
}

pub(crate) struct CleanupTask {
    session_states: ActiveLpSessions,
    cfg: LpDebug,
    shutdown: nym_task::ShutdownToken,
}

impl CleanupTask {
    pub fn new(
        session_states: ActiveLpSessions,
        cfg: LpDebug,
        shutdown: nym_task::ShutdownToken,
    ) -> Self {
        CleanupTask {
            session_states,
            cfg,
            shutdown,
        }
    }

    fn perform_cleanup(&self) {
        let session_ttl = self.cfg.session_ttl;

        let start = std::time::Instant::now();
        let mut ss_removed = 0u64;

        // Remove stale sessions (based on time since last activity)
        // Use shorter TTL for demoted (ReadOnlyTransport) sessions
        self.session_states.sessions.retain(|_, timestamped| {
            if timestamped.since_activity() > session_ttl {
                ss_removed += 1;
                false
            } else {
                true
            }
        });

        if ss_removed > 0 {
            let duration = start.elapsed();
            info!(
                "LP state cleanup: {ss_removed} sessions (took {:.3}s)",
                duration.as_secs_f64()
            );

            // Track metrics
            if ss_removed > 0 {
                inc_by!("lp_states_cleanup_session_removed", ss_removed as i64);
            }
        }
    }

    /// Background loop for cleaning up stale state entries
    ///
    /// Runs periodically to scan handshake_states and session_states maps,
    /// removing entries that have exceeded their TTL.
    ///
    /// Demoted sessions (ReadOnlyTransport) use shorter TTL since they
    /// only need to drain in-flight packets after subsession promotion.
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
