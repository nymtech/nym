// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::cleanup::Eviction;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::TimestampedState;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use dashmap::mapref::one::RefMut;
use nym_lp::LpTransportSession;
use nym_lp::session::{LpAction, LpInput};
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use nym_sphinx_addressing::ClientAddress;
use std::fmt::{Display, Formatter};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

/// Who a session belongs to.
///
/// Nodes are keyed by **IP, not socket address**: a session is established over the control port
/// and used over the data port, so the port cannot be part of a node's identity. Clients are keyed
/// by their [`ClientAddress`] - a client's address is not stable and is not how anything outside
/// the node refers to it.
///
/// [`ClientAddress`]: nym_sphinx_addressing::ClientAddress
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum LpPeer {
    Node(IpAddr),
    Client(ClientAddress),
}

impl LpPeer {
    pub fn node(ip: IpAddr) -> Self {
        LpPeer::Node(ip)
    }

    pub fn client(address: ClientAddress) -> Self {
        LpPeer::Client(address)
    }

    /// The one form the maps are indexed by, so equal peers always hash equal.
    fn normalised(self) -> Self {
        match self {
            // v4-mapped-v6 and plain v4 are the same peer
            LpPeer::Node(ip) => LpPeer::Node(ip.to_canonical()),
            client => client,
        }
    }
}

impl Display for LpPeer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LpPeer::Node(ip) => ip.fmt(f),
            LpPeer::Client(address) => address.fmt(f),
        }
    }
}

/// Established sessions keyed by the receiver index, with a secondary index by peer.
///
/// Nodes and clients share one store. The receiver index is what arrives on the wire and is only a
/// `u32`, so holding the two kinds apart would leave their index spaces independent: a collision
/// across them could not be detected at insert, and inbound traffic for one would be handed to the
/// other's session. Here a collision is caught by [`Self::insert_new_session`] the moment the
/// handshake completes. Entries are wrapped in [`TimestampedState`] for TTL-based cleanup.
///
/// # Several sessions may exist for one peer
///
/// `sessions` may hold any number of sessions for the same peer; `by_peer` names the one
/// currently used for *sending*. Establishing a new session **demotes** the previous one
/// rather than evicting it, so packets already in flight towards the old receiver index
/// still decrypt. Demoted sessions are read-only and expire on a shorter TTL.
///
/// # Lock order: `by_peer` before `sessions`
///
/// Sending resolves through both maps, and a session may be superseded and demoted at any moment,
/// so the two lookups have to be one atomic step - otherwise a sender can resolve an index, have it
/// demoted underneath, and lose the frame to a read-only session. Both
/// [`Self::with_sending_session_mut`] and [`Self::bind_peer`] therefore hold the `by_peer` guard
/// across their `sessions` access.
///
/// **Anything touching both maps must acquire them in that order.** Taking `sessions` first and
/// then reaching for `by_peer` while still holding it is a lock-order inversion and will deadlock.
/// [`Self::remove_stale`] touches both but never nests them, so it cannot take part in a cycle.
#[derive(Clone, Default)]
pub struct ActiveLpSessions {
    /// Primary store, keyed by what arrives on the wire.
    pub(crate) sessions: Arc<DashMap<LpReceiverIndex, TimestampedState<LpTransportSession>>>,

    /// Newest (sending) session per peer.
    by_peer: Arc<DashMap<LpPeer, LpReceiverIndex>>,

    /// Peer each addressed session belongs to, the reverse of `by_peer`.
    ///
    /// Packets name a session by receiver index, so this is what lets an inbound packet be
    /// attributed to a peer - a client's last-seen address cannot be refreshed otherwise, and
    /// [`Self::remove_stale`] could not tell which TTL applies.
    peer_by_index: Arc<DashMap<LpReceiverIndex, LpPeer>>,
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

    /// Insert a session no peer can yet be attached to.
    ///
    /// A client only names itself in its registration request, which arrives *over* the session
    /// it has just handshaked, so its session exists before its [`ClientAddress`] is known.
    /// Until [`Self::bind_peer`] names it, the session decrypts inbound packets but nothing can
    /// address it for sending.
    ///
    /// [`ClientAddress`]: nym_sphinx_addressing::ClientAddress
    pub(crate) fn insert_new_session(
        &self,
        session: LpTransportSession,
    ) -> Result<(), LpHandlerError> {
        let receiver_index = session.receiver_index();

        // Never clobber a live session: indices are derived with a per-handshake seed, so a
        // collision here means something is badly wrong rather than merely unlucky. Checked
        // through the entry API so two concurrent inserts of one index cannot both pass.
        match self.sessions.entry(receiver_index) {
            Entry::Occupied(_) => Err(LpHandlerError::DuplicateLpSession { receiver_index }),
            Entry::Vacant(slot) => {
                slot.insert(TimestampedState::new(session));
                Ok(())
            }
        }
    }

    /// Insert a session and make it the sending session for `peer`, demoting whatever was
    /// previously sending to that peer.
    pub fn insert_addressed_session(
        &self,
        peer: LpPeer,
        session: LpTransportSession,
    ) -> Result<(), LpHandlerError> {
        let receiver_index = session.receiver_index();
        self.insert_new_session(session)?;
        self.bind_peer(peer, receiver_index);
        Ok(())
    }

    /// Make `receiver_index` the sending session for `peer`, demoting whatever was previously
    /// sending to it.
    pub(crate) fn bind_peer(&self, peer: LpPeer, receiver_index: LpReceiverIndex) {
        let peer = peer.normalised();
        self.peer_by_index.insert(receiver_index, peer);

        // Repoint and demote under one `by_peer` guard, so a sender resolving this peer sees
        // either the old session while it is still live, or the new one - never the old one
        // after it has been demoted, which would cost it the frame it was carrying.
        //
        // This nests `sessions` inside `by_peer`; see the lock order on the struct.
        match self.by_peer.entry(peer) {
            Entry::Occupied(slot) => {
                let (_, previous) = slot.replace_entry(receiver_index);
                self.demote(previous);
            }
            Entry::Vacant(slot) => {
                slot.insert(receiver_index);
            }
        }
    }

    /// Peer a session belongs to, if it has been bound to one.
    pub(crate) fn peer_for(&self, receiver_index: LpReceiverIndex) -> Option<LpPeer> {
        self.peer_by_index
            .get(&receiver_index)
            .map(|entry| *entry.value())
    }

    /// Mark a session read-only. It keeps decrypting in-flight packets but can no longer send.
    fn demote(&self, receiver_index: LpReceiverIndex) {
        if let Some(mut entry) = self.sessions.get_mut(&receiver_index) {
            entry.value_mut().state.demote();
        }
    }

    /// Receiver index currently used for sending to `peer`, if any.
    pub(crate) fn sending_index_for(&self, peer: LpPeer) -> Option<LpReceiverIndex> {
        self.by_peer
            .get(&peer.normalised())
            .map(|entry| *entry.value())
    }

    pub(crate) fn has_session_for(&self, peer: LpPeer) -> bool {
        self.sending_index_for(peer).is_some()
    }

    /// Run `f` against a session, touching its activity timestamp.
    ///
    /// The closure form is deliberate: it makes it impossible to hold the `DashMap` guard
    /// across an await point, which would deadlock. Do not add an accessor that hands the
    /// guard out.
    pub(crate) fn with_session_mut<F, R>(
        &self,
        receiver_index: LpReceiverIndex,
        f: F,
    ) -> Result<R, LpHandlerError>
    where
        F: FnOnce(&mut LpTransportSession) -> R,
    {
        let mut entry = self
            .sessions
            .get_mut(&receiver_index)
            .ok_or(LpHandlerError::MissingLpSession { receiver_index })?;
        entry.value().touch();
        Ok(f(&mut entry.value_mut().state))
    }

    /// As [`Self::with_session_mut`], resolving the peer's current sending session.
    ///
    /// The `by_peer` guard is held for the whole call, making resolve-and-use atomic against
    /// [`Self::insert_session_inner`], which repoints and demotes under the same guard. A session
    /// can be superseded and demoted at any moment; a sender holding only the index it resolved
    /// would find it read-only and lose the frame it was carrying.
    ///
    /// This is a read guard, so concurrent senders do not block one another; only an insert for the
    /// same shard briefly excludes them.
    pub(crate) fn with_sending_session_mut<F, R>(
        &self,
        peer: LpPeer,
        f: F,
    ) -> Result<R, LpHandlerError>
    where
        F: FnOnce(&mut LpTransportSession) -> R,
    {
        let sending = self.by_peer.get(&peer.normalised()).ok_or_else(|| {
            LpHandlerError::NoSessionForPeer {
                peer: peer.to_string(),
            }
        })?;
        self.with_session_mut(*sending.value(), f)
    }

    /// Encrypt a frame on the session currently sending to `peer`.
    pub fn send_frame(
        &self,
        peer: LpPeer,
        frame: LpFrame,
    ) -> Result<EncryptedLpPacket, LpHandlerError> {
        let action =
            self.with_sending_session_mut(peer, |s| s.process_input(LpInput::SendFrame(frame)))??;

        match action {
            LpAction::SendPacket(packet) => Ok(packet),
            LpAction::DeliverFrame(_) => Err(LpHandlerError::UnexpectedLpAction),
        }
    }

    /// Decrypt a packet on the session it names, demoted or not.
    pub fn receive_packet(&self, packet: EncryptedLpPacket) -> Result<LpFrame, LpHandlerError> {
        let receiver_index = packet.outer_header().receiver_idx;
        let action = self.with_session_mut(receiver_index, |s| {
            s.process_input(LpInput::ReceivePacket(packet))
        })??;

        match action {
            LpAction::DeliverFrame(frame) => Ok(frame),
            LpAction::SendPacket(_) => Err(LpHandlerError::UnexpectedLpAction),
        }
    }

    /// Drop sessions idle beyond their TTL.
    ///
    /// Demoted sessions use `demoted_ttl`, since they only have to outlive packets already in
    /// flight. Otherwise the TTL is chosen by the kind of peer the session belongs to. A session
    /// with no peer at all is a client's that never completed registration, so it expires on the
    /// client TTL.
    pub(crate) fn remove_stale(
        &self,
        client_ttl: Duration,
        node_ttl: Duration,
        demoted_ttl: Duration,
    ) -> Eviction {
        let mut eviction = Eviction::default();
        let mut evicted = Vec::new();

        self.sessions.retain(|receiver_index, timestamped| {
            let demoted = timestamped.state.is_read_only();
            let ttl = if demoted {
                demoted_ttl
            } else {
                match self.peer_for(*receiver_index) {
                    Some(LpPeer::Node(_)) => node_ttl,
                    Some(LpPeer::Client(_)) | None => client_ttl,
                }
            };

            if timestamped.since_activity() > ttl {
                if demoted {
                    eviction.demoted_removed += 1;
                } else {
                    eviction.live_removed += 1;
                }
                evicted.push(*receiver_index);
                false
            } else {
                true
            }
        });

        for receiver_index in evicted {
            let Some((_, peer)) = self.peer_by_index.remove(&receiver_index) else {
                continue;
            };
            // the peer may already have been repointed at a newer session, which must survive.
            // that this removed anything is precisely the signal that nothing can reach the peer
            // any more - a demoted session expiring under a live one reports nothing.
            if self
                .by_peer
                .remove_if(&peer, |_, sending| *sending == receiver_index)
                .is_some()
            {
                eviction.forgotten_peers.push(peer);
            }
        }

        eviction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_lp::SessionsMock;
    use std::net::Ipv4Addr;

    /// Anything older than this counts as expired below.
    const SHORT: Duration = Duration::from_secs(1);
    const FOREVER: Duration = Duration::from_secs(86_400);
    const AGED: Duration = Duration::from_secs(60);

    /// Age every session, so a TTL test does not have to sleep for one.
    fn age_everything(sessions: &ActiveLpSessions) {
        for entry in sessions.sessions.iter() {
            entry.value().backdate(AGED);
        }
    }

    fn node_peer(n: u8) -> LpPeer {
        LpPeer::node(IpAddr::V4(Ipv4Addr::new(127, 0, 0, n)))
    }

    fn client_peer(n: u8) -> LpPeer {
        LpPeer::client(ClientAddress::from_bytes([n; 20]))
    }

    fn session(seed: u64) -> LpTransportSession {
        SessionsMock::mock_seeded_post_handshake(seed, nym_lp::KEM::MlKem768).initiator
    }

    /// A node and a client live in one store, each reachable only by its own key.
    ///
    /// The variants cannot collide, so nothing keyed by IP can ever resolve a client, or the
    /// reverse - which is the property that lets the two share a store at all.
    #[test]
    fn node_and_client_sessions_coexist() {
        let sessions = ActiveLpSessions::new();
        let (node, client) = (node_peer(1), client_peer(1));

        let node_index = session(1).receiver_index();
        let client_index = session(2).receiver_index();
        sessions.insert_addressed_session(node, session(1)).unwrap();
        sessions
            .insert_addressed_session(client, session(2))
            .unwrap();

        assert_eq!(sessions.sending_index_for(node), Some(node_index));
        assert_eq!(sessions.sending_index_for(client), Some(client_index));
        assert_eq!(sessions.peer_for(node_index), Some(node));
        assert_eq!(sessions.peer_for(client_index), Some(client));
    }

    /// The same receiver index from two kinds of peer is refused, rather than one shadowing the
    /// other.
    ///
    /// The index is a `u32` drawn per handshake, so collisions are a birthday problem, not an
    /// impossibility. Held in separate stores this could not be noticed at insert and would instead
    /// surface as a peer's traffic failing to decrypt for as long as both sessions lived.
    #[test]
    fn a_colliding_index_is_refused_across_kinds() {
        let sessions = ActiveLpSessions::new();

        // the same seed yields the same receiver index
        let index = session(7).receiver_index();
        sessions
            .insert_addressed_session(node_peer(1), session(7))
            .unwrap();

        let err = sessions
            .insert_addressed_session(client_peer(1), session(7))
            .unwrap_err();

        assert!(matches!(
            err,
            LpHandlerError::DuplicateLpSession { receiver_index } if receiver_index == index
        ));
    }

    /// A node and a client expire under their own TTLs, chosen per entry.
    #[test]
    fn each_kind_expires_on_its_own_ttl() {
        let sessions = ActiveLpSessions::new();
        sessions
            .insert_addressed_session(node_peer(1), session(1))
            .unwrap();
        sessions
            .insert_addressed_session(client_peer(1), session(2))
            .unwrap();

        age_everything(&sessions);

        // clients expire, nodes never
        let eviction = sessions.remove_stale(SHORT, FOREVER, FOREVER);

        assert_eq!(eviction.live_removed, 1);
        assert_eq!(eviction.forgotten_peers, vec![client_peer(1)]);
        assert!(sessions.has_session_for(node_peer(1)));
        assert!(!sessions.has_session_for(client_peer(1)));
    }

    /// A demoted session expiring does not forget its peer - the live one that superseded it is
    /// still there.
    ///
    /// This is what the address registry keys off, so reporting the peer here would strand a client
    /// that is perfectly reachable.
    #[test]
    fn a_demoted_session_expiring_does_not_forget_its_peer() {
        let sessions = ActiveLpSessions::new();
        let client = client_peer(1);

        sessions
            .insert_addressed_session(client, session(1))
            .unwrap();
        // supersede it, demoting the first
        sessions
            .insert_addressed_session(client, session(2))
            .unwrap();

        age_everything(&sessions);

        // only demoted sessions expire
        let eviction = sessions.remove_stale(FOREVER, FOREVER, SHORT);

        assert_eq!(eviction.demoted_removed, 1);
        assert_eq!(eviction.live_removed, 0);
        assert!(
            eviction.forgotten_peers.is_empty(),
            "the peer still has a live session"
        );
        assert!(sessions.has_session_for(client));
    }

    /// A session never bound to a peer expires on the client TTL.
    ///
    /// That is what it is: a client that handshaked and never got as far as registering.
    #[test]
    fn an_unaddressed_session_expires_as_a_client() {
        let sessions = ActiveLpSessions::new();
        sessions.insert_new_session(session(1)).unwrap();
        age_everything(&sessions);

        let eviction = sessions.remove_stale(SHORT, FOREVER, FOREVER);

        assert_eq!(eviction.live_removed, 1);
        assert!(eviction.forgotten_peers.is_empty(), "it never had a peer");
    }

    /// v4-mapped-v6 and plain v4 are the same node however the key was built.
    #[test]
    fn node_addresses_are_canonicalised() {
        let sessions = ActiveLpSessions::new();
        let v4 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let mapped = IpAddr::V6(Ipv4Addr::new(127, 0, 0, 1).to_ipv6_mapped());

        sessions
            .insert_addressed_session(LpPeer::Node(mapped), session(1))
            .unwrap();

        assert!(sessions.has_session_for(LpPeer::node(v4)));
    }
}
