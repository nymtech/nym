// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpConfig;
use crate::node::lp::cleanup::TimestampedState;
use crate::node::lp::directory::LpNodes;
use crate::node::lp::error::LpHandlerError;
use dashmap::DashMap;
use dashmap::mapref::one::RefMut;
use nym_gateway::node::wireguard::PeerRegistrator;
use nym_lp::LpTransportSession;
use nym_lp::peer::LpLocalPeer;
use nym_lp::peer_config::LpReceiverIndex;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_node_metrics::NymNodeMetrics;
use std::sync::Arc;
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

    /// Common shared data
    pub shared: SharedLpState,
}

/// [Placeholder] Shared state for LP nodes control connections
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

/// Established sessions keyed by the receiver index
///
/// Wrapped in TimestampedState for TTL-based cleanup of inactive sessions.
#[derive(Clone, Default)]
pub struct ActiveLpSessions {
    // TODO: this might require split between client and node sessions. TBD
    pub(crate) sessions: Arc<DashMap<LpReceiverIndex, TimestampedState<LpTransportSession>>>,
}

impl ActiveLpSessions {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get_state_entry_mut(
        &self,
        receiver_index: LpReceiverIndex,
    ) -> Result<RefMut<'_, LpReceiverIndex, TimestampedState<LpTransportSession>>, LpHandlerError>
    {
        self.sessions
            .get_mut(&receiver_index)
            .ok_or_else(|| LpHandlerError::MissingLpSession { receiver_index })
    }

    pub(crate) fn insert_new_session(&self, session: LpTransportSession) {
        let receiver_index = session.receiver_index();
        self.sessions
            .insert(receiver_index, TimestampedState::new(session));
    }
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
