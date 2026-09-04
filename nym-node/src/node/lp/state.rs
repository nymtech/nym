// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpConfig;
use crate::node::lp::active_sessions::ActiveLpSessions;
use crate::node::lp::directory::LpNodes;
use nym_gateway::node::wireguard::PeerRegistrator;
use nym_lp::peer::LpLocalPeer;
use nym_node_metrics::NymNodeMetrics;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Shared state for LP control connections
#[derive(Clone)]
pub struct SharedLpClientControlState {
    /// Encapsulates all required key information of a local Lewes Protocol Peer.
    pub local_lp_peer: LpLocalPeer,

    /// Handle registering new wireguard peers
    pub peer_registrator: Option<PeerRegistrator>,

    /// Semaphore limiting concurrent forward connections
    ///
    /// Prevents file descriptor exhaustion when forwarding LP packets during
    /// telescope setup. When at capacity, forward requests return an error
    /// so clients can choose a different gateway.
    // this is temporary until there is persistent KKT/PSQ session between nodes
    pub forward_semaphore: Arc<Semaphore>,

    /// Currently active LP sessions
    pub session_states: ActiveLpSessions,

    /// Common shared data
    pub shared: SharedLpState,
}

/// Shared state for LP node-to-node control connections
#[derive(Clone)]
pub struct SharedLpNodeControlState {
    /// Encapsulates all required key information of a local Lewes Protocol Peer.
    pub local_lp_peer: LpLocalPeer,

    /// Information about all known LP nodes
    pub nodes: LpNodes,

    /// Sessions established with other nym-nodes.
    ///
    /// Separate key space from client sessions, and shared with the data plane — the control
    /// plane's only job is to complete a handshake and deposit the session here.
    pub node_sessions: ActiveLpSessions,

    /// Common shared data
    pub shared: SharedLpState,
}

/// Shared state for LP connection handlers
#[derive(Clone)]
pub struct SharedLpState {
    /// Metrics collection
    pub metrics: NymNodeMetrics,

    /// LP configuration (for timestamp validation, etc.)
    pub lp_config: LpConfig,
}

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
