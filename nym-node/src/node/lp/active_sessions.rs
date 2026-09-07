// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::TimestampedState;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use dashmap::mapref::one::RefMut;
use nym_lp::LpTransportSession;
use nym_lp::session::{LpAction, LpInput};
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

/// Established sessions keyed by the receiver index.
///
/// One type, used for two separate key spaces: client sessions (one instance, `by_addr`
/// unused) and node-to-node sessions (a second instance, `by_addr` populated). Entries are
/// wrapped in [`TimestampedState`] for TTL-based cleanup. // SW Needs to change when accomodating clients
///
/// # Several sessions may exist for one peer
///
/// `sessions` may hold any number of sessions for the same peer; `by_addr` names the one
/// currently used for *sending*. Establishing a new session **demotes** the previous one
/// rather than evicting it, so packets already in flight towards the old receiver index
/// still decrypt. Demoted sessions are read-only and expire on a shorter TTL.
///
/// # Lock order: `by_addr` before `sessions`
///
/// Sending resolves through both maps, and a session may be superseded and demoted at any moment,
/// so the two lookups have to be one atomic step - otherwise a sender can resolve an index, have it
/// demoted underneath, and lose the frame to a read-only session. Both
/// [`Self::with_sending_session_mut`] and [`Self::insert_session_inner`] therefore hold the
/// `by_addr` guard across their `sessions` access.
///
/// **Anything touching both maps must acquire them in that order.** Taking `sessions` first and
/// then reaching for `by_addr` while still holding it is a lock-order inversion and will deadlock.
/// [`Self::remove_stale`] touches both but never nests them, so it cannot take part in a cycle.
#[derive(Clone, Default)]
pub struct ActiveLpSessions {
    /// Primary store, keyed by what arrives on the wire.
    pub(crate) sessions: Arc<DashMap<LpReceiverIndex, TimestampedState<LpTransportSession>>>,

    /// Newest (sending) session per peer. Empty for client sessions; populated for node
    /// sessions, where the data plane resolves a next-hop address rather than an index.
    by_addr: Arc<DashMap<IpAddr, LpReceiverIndex>>,
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

    /// Insert a session that is not associated with a peer address (the client path).
    pub(crate) fn insert_new_session(
        &self,
        session: LpTransportSession,
    ) -> Result<(), LpHandlerError> {
        self.insert_session_inner(session, None)
    }

    /// Insert a node-to-node session and make it the sending session for `peer_ip`,
    /// demoting whatever was previously sending to that peer.
    pub fn insert_node_session(
        &self,
        peer_ip: IpAddr,
        session: LpTransportSession,
    ) -> Result<(), LpHandlerError> {
        self.insert_session_inner(session, Some(peer_ip.to_canonical()))
    }

    fn insert_session_inner(
        &self,
        session: LpTransportSession,
        peer_ip: Option<IpAddr>,
    ) -> Result<(), LpHandlerError> {
        let receiver_index = session.receiver_index();

        // Never clobber a live session: indices are derived with a per-handshake seed, so a
        // collision here means something is badly wrong rather than merely unlucky. Checked
        // through the entry API so two concurrent inserts of one index cannot both pass.
        match self.sessions.entry(receiver_index) {
            Entry::Occupied(_) => {
                return Err(LpHandlerError::DuplicateLpSession { receiver_index });
            }
            Entry::Vacant(slot) => {
                slot.insert(TimestampedState::new(session));
            }
        }

        if let Some(peer_ip) = peer_ip {
            // Repoint and demote under one `by_addr` guard, so a sender resolving this peer sees
            // either the old session while it is still live, or the new one - never the old one
            // after it has been demoted, which would cost it the frame it was carrying.
            //
            // This nests `sessions` inside `by_addr`; see the lock order on the struct.
            match self.by_addr.entry(peer_ip) {
                Entry::Occupied(slot) => {
                    let (_, previous) = slot.replace_entry(receiver_index);
                    self.demote(previous);
                }
                Entry::Vacant(slot) => {
                    slot.insert(receiver_index);
                }
            }
        }

        Ok(())
    }

    /// Mark a session read-only. It keeps decrypting in-flight packets but can no longer send.
    fn demote(&self, receiver_index: LpReceiverIndex) {
        if let Some(mut entry) = self.sessions.get_mut(&receiver_index) {
            entry.value_mut().state.demote();
        }
    }

    /// Receiver index currently used for sending to `peer_ip`, if any.
    pub(crate) fn sending_index_for(&self, peer_ip: IpAddr) -> Option<LpReceiverIndex> {
        self.by_addr
            .get(&peer_ip.to_canonical())
            .map(|entry| *entry.value())
    }

    pub(crate) fn has_session_for(&self, peer_ip: IpAddr) -> bool {
        self.sending_index_for(peer_ip).is_some()
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
    /// The `by_addr` guard is held for the whole call, making resolve-and-use atomic against
    /// [`Self::insert_session_inner`], which repoints and demotes under the same guard. A session
    /// can be superseded and demoted at any moment; a sender holding only the index it resolved
    /// would find it read-only and lose the frame it was carrying.
    ///
    /// This is a read guard, so concurrent senders do not block one another; only an insert for the
    /// same shard briefly excludes them.
    pub(crate) fn with_sending_session_mut<F, R>(
        &self,
        peer_ip: IpAddr,
        f: F,
    ) -> Result<R, LpHandlerError>
    where
        F: FnOnce(&mut LpTransportSession) -> R,
    {
        let sending = self
            .by_addr
            .get(&peer_ip.to_canonical())
            .ok_or(LpHandlerError::NoSessionForPeer { peer_ip })?;
        self.with_session_mut(*sending.value(), f)
    }

    /// Encrypt a frame on the session currently sending to `peer_ip`.
    pub(crate) fn send_frame(
        &self,
        peer_ip: IpAddr,
        frame: LpFrame,
    ) -> Result<EncryptedLpPacket, LpHandlerError> {
        let action = self
            .with_sending_session_mut(peer_ip, |s| s.process_input(LpInput::SendFrame(frame)))??;

        match action {
            LpAction::SendPacket(packet) => Ok(packet),
            LpAction::DeliverFrame(_) => Err(LpHandlerError::UnexpectedLpAction),
        }
    }

    /// Decrypt a packet on the session it names, demoted or not.
    pub(crate) fn receive_packet(
        &self,
        packet: EncryptedLpPacket,
    ) -> Result<LpFrame, LpHandlerError> {
        let receiver_index = packet.outer_header().receiver_idx;
        let action = self.with_session_mut(receiver_index, |s| {
            s.process_input(LpInput::ReceivePacket(packet))
        })??;

        match action {
            LpAction::DeliverFrame(frame) => Ok(frame),
            LpAction::SendPacket(_) => Err(LpHandlerError::UnexpectedLpAction),
        }
    }

    /// Drop sessions idle beyond their TTL. Demoted sessions use `demoted_ttl`, since they
    /// only have to outlive packets already in flight.
    ///
    /// Returns `(live_removed, demoted_removed)`.
    pub(crate) fn remove_stale(&self, session_ttl: Duration, demoted_ttl: Duration) -> (u64, u64) {
        let mut live_removed = 0;
        let mut demoted_removed = 0;
        let mut evicted = Vec::new();

        self.sessions.retain(|receiver_index, timestamped| {
            let demoted = timestamped.state.is_read_only();
            let ttl = if demoted { demoted_ttl } else { session_ttl };

            if timestamped.since_activity() > ttl {
                if demoted {
                    demoted_removed += 1;
                } else {
                    live_removed += 1;
                }
                evicted.push(*receiver_index);
                false
            } else {
                true
            }
        });

        // only clear a peer mapping that still points at something we just evicted
        for receiver_index in evicted {
            self.by_addr.retain(|_, sending| *sending != receiver_index);
        }

        (live_removed, demoted_removed)
    }
}
