// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpConfig;
use crate::node::lp::active_sessions::ActiveLpSessions;
use crate::node::lp::directory::LpNodes;
use crate::node::lp::registration_nonces::RegistrationNonces;
use nym_crypto::asymmetric::ed25519;
use nym_gateway::node::wireguard::PeerRegistrator;
use nym_lp::peer::LpLocalPeer;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_node_metrics::NymNodeMetrics;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Shared state for LP control connections, held by every node whatever it serves.
///
/// Everything here belongs to the node itself rather than to anything it talks to, so the node and
/// client handlers both work from this one copy. What only a gateway has lives in
/// [`SharedGatewayLpControlState`] instead.
///
/// The line between the two is ownership, not spelling: a fact about this node belongs to exactly
/// one of them, while a handle to a store neither of them owns - [`ActiveLpSessions`], which the
/// data plane and the cleanup task hold as well - may appear in both.
///
/// Cheap to clone: every member is a handle to state shared with the data plane, not the state
/// itself.
#[derive(Clone)]
pub struct SharedLpControlState {
    /// Encapsulates all required key information of a local Lewes Protocol Peer.
    pub local_lp_peer: LpLocalPeer,

    /// Information about all known LP nodes, which is how an incoming connection is recognised as
    /// one of theirs and how an outgoing one is addressed.
    pub nodes: LpNodes,

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

/// Gateway-only shared state for LP control connections.
///
/// Only a node that forwards client traffic has any of this, so it is built nowhere else and the
/// listener holds it as an `Option`: a mixnode has nothing to serve a client connection with and
/// refuses it, rather than completing a handshake, accepting a registration, and then dropping
/// every packet that followed.
#[derive(Clone)]
pub struct SharedGatewayLpControlState {
    /// This node's ed25519 identity. A mixnet registration is signed for the node it is meant
    /// for, and only verifies here if that is this one - which is the only thing that reads it,
    /// so it lives on the side that serves clients rather than being reachable from both.
    pub identity: ed25519::PublicKey,

    /// Every established LP session, clients and nodes alike - the same store
    /// [`SharedLpControlState`] holds, since registration's whole job is to give one of those
    /// sessions a name.
    pub sessions: ActiveLpSessions,

    /// Challenges issued to clients part-way through a mixnet registration.
    pub registration_nonces: RegistrationNonces,

    /// Handle registering new wireguard peers
    pub peer_registrator: Option<PeerRegistrator>,

    /// Semaphore limiting concurrent forward connections
    ///
    /// Prevents file descriptor exhaustion when forwarding LP packets during
    /// telescope setup. When at capacity, forward requests return an error
    /// so clients can choose a different gateway.
    // this is temporary until there is persistent KKT/PSQ session between nodes
    pub forward_semaphore: Arc<Semaphore>,
}

impl SharedGatewayLpControlState {
    pub(crate) fn new(
        identity: ed25519::PublicKey,
        sessions: ActiveLpSessions,
        peer_registrator: Option<PeerRegistrator>,
        lp_config: &LpConfig,
    ) -> Self {
        SharedGatewayLpControlState {
            identity,
            sessions,
            registration_nonces: RegistrationNonces::new(),
            peer_registrator,
            forward_semaphore: Arc::new(Semaphore::new(lp_config.debug.max_concurrent_forwards)),
        }
    }
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
    /// Metrics collection
    pub metrics: NymNodeMetrics,

    /// LP configuration (for timestamp validation, etc.)
    pub lp_config: LpConfig,

    /// Currently active LP sessions
    pub session_states: ActiveLpSessions,
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
