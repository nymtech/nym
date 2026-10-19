// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpConfig;
use crate::node::lp::active_sessions::ActiveLpSessions;
use crate::node::lp::directory::LpNodes;
use nym_crypto::asymmetric::ed25519;
use nym_gateway::node::wireguard::PeerRegistrator;
use nym_lp::peer::LpLocalPeer;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_node_metrics::NymNodeMetrics;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Shared state for LP control connections
#[derive(Clone)]
pub struct SharedLpClientControlState {
    /// Encapsulates all required key information of a local Lewes Protocol Peer.
    pub local_lp_peer: LpLocalPeer,

    /// This node's ed25519 identity. A mixnet registration is signed for the node it is meant
    /// for, and only verifies here if that is this one.
    pub identity: ed25519::PublicKey,

    /// Handle registering new wireguard peers
    pub peer_registrator: Option<PeerRegistrator>,

    /// Semaphore limiting concurrent forward connections
    ///
    /// Prevents file descriptor exhaustion when forwarding LP packets during
    /// telescope setup. When at capacity, forward requests return an error
    /// so clients can choose a different gateway.
    // this is temporary until there is persistent KKT/PSQ session between nodes
    pub forward_semaphore: Arc<Semaphore>,

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

    /// Common shared data
    pub shared: SharedLpState,
}

/// Shared state for LP data connections
#[derive(Clone)]
pub struct SharedLpDataState {
    /// Channel for forwarding Sphinx packets into the mixnet
    ///
    /// Used by the LP data handler (UDP:51264) to forward decrypted Sphinx packets
    /// from LP clients into the mixnet for routing.
    #[allow(dead_code)]
    pub outbound_mix_sender: MixForwardingSender,

    /// Common shared data
    pub shared: SharedLpState,
}

/// Shared state for LP connection handlers
#[derive(Clone)]
pub struct SharedLpState {
    /// Every established LP session, clients and nodes alike.
    ///
    /// One store, held by both control handlers and the data plane: the control plane's only job
    /// is to complete a handshake and deposit the session here.
    pub sessions: ActiveLpSessions,

    /// Metrics collection
    pub metrics: NymNodeMetrics,

    /// LP configuration (for timestamp validation, etc.)
    pub lp_config: LpConfig,
}

/// Wrapper for state entries with timestamp tracking for cleanup
///
/// This wrapper adds a `last_activity` timestamp to state entries,
/// enabling TTL-based cleanup of stale sessions.
pub struct TimestampedState<T> {
    /// The actual state (LpStateMachine or LpSession)
    pub state: T,

    /// Last activity timestamp (unix seconds, atomically updated on every packet received)
    last_activity: std::sync::atomic::AtomicU64,
}

impl<T> TimestampedState<T> {
    /// Create a new timestamped state
    pub fn new(state: T) -> Self {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            state,
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

    /// Pretend the last activity was `by` ago.
    ///
    /// [`Self::since_activity`] has whole-second resolution, so a TTL test cannot otherwise reach
    /// an expired state without actually sleeping.
    #[cfg(test)]
    pub(crate) fn backdate(&self, by: Duration) {
        let last = self
            .last_activity
            .load(std::sync::atomic::Ordering::Relaxed);
        self.last_activity.store(
            last.saturating_sub(by.as_secs()),
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
