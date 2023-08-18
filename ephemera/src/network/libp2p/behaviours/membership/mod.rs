//! In Ephemera, membership of reliable broadcast protocol is decided by membership provider.
//! Only peers who are returned by [`crate::membership::MembersProviderFut`] are allowed to participate.

use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;

use libp2p_identity::PeerId;
use lru::LruCache;

use crate::network::Peer;

pub(crate) mod behaviour;
mod connections;
mod handler;
mod protocol;

const MAX_DIAL_ATTEMPT_ROUNDS: usize = 6;

/// Minimum percentage of available nodes to consider the network healthy.
//TODO: make this configurable
const MEMBERSHIP_MINIMUM_AVAILABLE_NODES_RATIO: f64 = 0.8;

/// Minimum time between syncs of membership.
const MEMBERSHIP_SYNC_INTERVAL_SEC: u64 = 60;

/// Maximum percentage of nodes that can change in a single membership update.
/// In general it should be considered a security risk if it has changed too much.
/// //TODO: make this configurable
const _MEMBERSHIP_MAXIMUM_ALLOWED_CHANGE_RATIO: f64 = 0.2;

/// Membership provider returns list of peers. But it is up to the Ephemera user to decide
/// how reliable the list is. For example, it can contain peers who are offline.

/// This enum defines how the actual membership is decided.
#[derive(Debug)]
pub(crate) enum MembershipKind {
    /// Mandatory minimum membership size is defined by threshold of all peers returned by membership provider.
    Threshold(f64),
    /// Mandatory minimum membership size is all peers who are online.
    AnyOnline,
    /// Mandatory minimum membership size is all peers returned by membership provider.
    AllOnline,
}

impl MembershipKind {
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    pub(crate) fn accept(&self, membership: &Membership) -> bool {
        let total_number_of_peers = membership.all_members.len();
        let connected_peers = membership.connected_peers_ids.len();
        match self {
            MembershipKind::Threshold(threshold) => {
                let minimum_available_nodes = (total_number_of_peers as f64 * threshold) as usize;
                connected_peers >= minimum_available_nodes
            }
            MembershipKind::AnyOnline => connected_peers > 0,
            MembershipKind::AllOnline => connected_peers == total_number_of_peers,
        }
    }
}

pub(crate) struct Memberships {
    snapshots: LruCache<u64, Membership>,
    current: u64,
    /// This is set when we get new peers set from [crate::membership::MembersProviderFut]
    /// but haven't yet activated it.
    pending_membership: Option<Membership>,
}

impl Memberships {
    pub(crate) fn new() -> Self {
        let mut snapshots = LruCache::new(NonZeroUsize::new(1000).unwrap());
        snapshots.put(0, Membership::new(HashMap::default()));
        Self {
            snapshots,
            current: 0,
            pending_membership: None,
        }
    }

    pub(crate) fn current(&mut self) -> &Membership {
        //Unwrap is safe because we always have current membership
        self.snapshots.get(&self.current).unwrap()
    }

    pub(crate) fn update(&mut self, membership: Membership) {
        self.current += 1;
        self.snapshots.put(self.current, membership);
    }

    pub(crate) fn set_pending(&mut self, membership: Membership) {
        self.pending_membership = Some(membership);
    }

    pub(crate) fn remove_pending(&mut self) -> Option<Membership> {
        self.pending_membership.take()
    }

    pub(crate) fn pending(&self) -> Option<&Membership> {
        self.pending_membership.as_ref()
    }

    pub(crate) fn pending_mut(&mut self) -> Option<&mut Membership> {
        self.pending_membership.as_mut()
    }
}

#[derive(Debug)]
pub(crate) struct Membership {
    local_peer_id: PeerId,
    all_members: HashMap<PeerId, Peer>,
    all_peers_ids: HashSet<PeerId>,
    connected_peers_ids: HashSet<PeerId>,
}

impl Membership {
    pub(crate) fn new_with_local(
        all_members: HashMap<PeerId, Peer>,
        local_peer_id: PeerId,
    ) -> Self {
        let all_peers_ids = all_members.keys().copied().collect();
        Self {
            local_peer_id,
            all_members,
            all_peers_ids,
            connected_peers_ids: HashSet::new(),
        }
    }

    pub(crate) fn new(all_members: HashMap<PeerId, Peer>) -> Self {
        let all_peers_ids = all_members.keys().copied().collect();
        Self {
            local_peer_id: PeerId::random(),
            all_members,
            all_peers_ids,
            connected_peers_ids: HashSet::new(),
        }
    }

    pub(crate) fn includes_local(&self) -> bool {
        self.all_members.contains_key(&self.local_peer_id)
    }

    pub(crate) fn peer_connected(&mut self, peer_id: PeerId) {
        self.connected_peers_ids.insert(peer_id);
    }

    pub(crate) fn peer_disconnected(&mut self, peer_id: &PeerId) {
        self.connected_peers_ids.remove(peer_id);
    }

    pub(crate) fn all_peer_ids(&self) -> &HashSet<PeerId> {
        &self.all_peers_ids
    }

    pub(crate) fn connected_peer_ids(&self) -> HashSet<PeerId> {
        self.connected_peers_ids.clone()
    }

    pub(crate) fn connected_peer_ids_with_local(&self) -> HashSet<PeerId> {
        let mut active_peers = self.connected_peers_ids.clone();
        active_peers.insert(self.local_peer_id);
        active_peers
    }

    pub(crate) fn connected_peers(&self) -> &HashSet<PeerId> {
        &self.connected_peers_ids
    }

    pub(crate) fn peer_address(&self, peer_id: &PeerId) -> Option<&libp2p::Multiaddr> {
        self.all_members
            .get(peer_id)
            .map(|peer| peer.address.inner())
    }
}

impl From<crate::config::MembershipKind> for MembershipKind {
    fn from(kind: crate::config::MembershipKind) -> Self {
        match kind {
            crate::config::MembershipKind::Threshold => {
                MembershipKind::Threshold(MEMBERSHIP_MINIMUM_AVAILABLE_NODES_RATIO)
            }
            crate::config::MembershipKind::AnyOnline => MembershipKind::AnyOnline,
            crate::config::MembershipKind::AllOnline => MembershipKind::AllOnline,
        }
    }
}
