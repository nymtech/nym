// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Frames waiting for their scheduled send time, and the rules for when they leave or are dropped.

use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use nym_lp_data::packet::LpFrame;
use nym_lp_data::{AddressedTimedData, TimedData};

/// Per-peer ceiling on queued frames, matching the legacy mixnet client's
/// `maximum_connection_buffer_size` - a per-peer packet buffer already tuned in production.
const MAX_QUEUED_PER_PEER: usize = 192;

/// Frames awaiting their scheduled send time, queued per peer in release order.
///
/// A peer is an *identity*, not a place: a node by its address, a client by its
/// [`ClientAddress`](nym_sphinx_addressing::ClientAddress). That is what selects the session
/// encrypting the frame, and it is stable while a client's socket address is not - the wire
/// address is looked up only when the frame is finally wrapped.
///
/// Held un-encrypted: the LP counter is assigned by the transport wrap and is cleartext on the wire,
/// so it must be allocated in transmission order. Queueing per peer is what makes that order cheap
/// to maintain, since each peer is a separate session with its own counter space and an observer
/// sees only one link's sequence.
///
/// A peer with no session keeps its frames here rather than moving them elsewhere: "stalled" is a
/// position in time, not a place. They leave by the ordinary due-drain once the session exists, or
/// are discarded by [`Self::drop_stalled`].
///
/// The queued frames are *not* addressed: the key already says where they go, so carrying a
/// destination per frame would be the same value repeated up to [`MAX_QUEUED_PER_PEER`] times.
/// [`Self::take_due`] re-attaches it on the way out.
#[derive(Default)]
pub(crate) struct OutgoingFrames {
    by_peer: HashMap<NymNodeRoutingAddress, VecDeque<TimedData<LpFrame>>>,
}

impl OutgoingFrames {
    /// Queue a frame in its peer's release order, dropping it if that peer is at its ceiling.
    ///
    /// Returns whether the frame was dropped.
    ///
    /// Workers run concurrently and `mix()` draws an independent delay per packet, so the order
    /// frames are produced in bears no relation to the order they must be sent in. Sorting on insert
    /// is what lets the drain simply take from the front. Frames sharing a release time keep the
    /// order the workers produced them in.
    ///
    /// A full queue rejects the arrival rather than evicting: every frame already queued has a
    /// release time this node has committed to, and the arrival is the only one that does not yet.
    pub(crate) fn queue(
        &mut self,
        frame: AddressedTimedData<LpFrame, NymNodeRoutingAddress>,
    ) -> bool {
        let AddressedTimedData { data, dst, .. } = frame;
        let queue = self.by_peer.entry(dst).or_default();

        if queue.len() >= MAX_QUEUED_PER_PEER {
            return true;
        }

        let at = queue.partition_point(|queued| queued.timestamp <= data.timestamp);
        queue.insert(at, data);

        false
    }

    /// Every peer with at least one frame queued.
    ///
    /// Collected rather than borrowed so the caller can consult the session store and the dialer
    /// while acting on each peer.
    pub(crate) fn peers(&self) -> Vec<NymNodeRoutingAddress> {
        self.by_peer.keys().copied().collect()
    }

    /// Whether `peer` has a frame whose scheduled send time has arrived.
    pub(crate) fn has_due(&self, peer: NymNodeRoutingAddress, now: Instant) -> bool {
        self.by_peer
            .get(&peer)
            .and_then(|queue| queue.front())
            .is_some_and(|frame| frame.timestamp <= now)
    }

    /// Take every frame of `peer`'s whose scheduled send time has arrived, in release order.
    ///
    /// The destination comes back from the key, which is the only place it was ever kept.
    pub(crate) fn take_due(
        &mut self,
        peer: NymNodeRoutingAddress,
        now: Instant,
    ) -> Vec<AddressedTimedData<LpFrame, NymNodeRoutingAddress>> {
        let Some(queue) = self.by_peer.get_mut(&peer) else {
            return Vec::new();
        };

        let due = queue.partition_point(|frame| frame.timestamp <= now);
        queue
            .drain(..due)
            .map(|frame| AddressedTimedData::new_addressed(frame.timestamp, frame.data, peer))
            .collect()
    }

    /// Discard `peer`'s frames that have waited past `timeout`, returning how many.
    ///
    /// Dropping is not only about memory. A frame emitted long past its schedule is an anomalous
    /// timing event: the delay it carries was drawn from a distribution the mixing depends on, so
    /// releasing it seconds late puts a packet on the wire at a time no delay would have produced.
    pub(crate) fn drop_stalled(
        &mut self,
        peer: NymNodeRoutingAddress,
        now: Instant,
        timeout: Duration,
    ) -> usize {
        let Some(queue) = self.by_peer.get_mut(&peer) else {
            return 0;
        };

        let stalled =
            queue.partition_point(|frame| now.saturating_duration_since(frame.timestamp) > timeout);
        queue.drain(..stalled).count()
    }

    /// Forget peers with nothing queued, so an entry does not outlive the traffic that created it.
    pub(crate) fn prune_empty(&mut self) {
        self.by_peer.retain(|_, queue| !queue.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_lp_data::packet::frame::LpFrameKind;
    use nym_sphinx_addressing::ClientAddress;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    const TIMEOUT: Duration = Duration::from_secs(5);

    fn peer(n: u8) -> NymNodeRoutingAddress {
        NymNodeRoutingAddress::Node(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, n)),
            51264,
        ))
    }

    fn client_peer(n: u8) -> NymNodeRoutingAddress {
        NymNodeRoutingAddress::Client(ClientAddress::from_bytes([n; 20]))
    }

    fn frame_for(
        dst: NymNodeRoutingAddress,
        seq: u8,
        at: Instant,
    ) -> AddressedTimedData<LpFrame, NymNodeRoutingAddress> {
        AddressedTimedData::new_addressed(at, LpFrame::new(LpFrameKind::Opaque, vec![seq]), dst)
    }

    fn frame(seq: u8, at: Instant) -> AddressedTimedData<LpFrame, NymNodeRoutingAddress> {
        frame_for(peer(1), seq, at)
    }

    fn seq_of(frames: &[AddressedTimedData<LpFrame, NymNodeRoutingAddress>]) -> Vec<u8> {
        frames.iter().map(|f| f.data.data.content[0]).collect()
    }

    /// Frames queue in scheduled order however the workers happened to produce them.
    ///
    /// This is what keeps LP counters monotonic on the wire: the counter is assigned by the wrap
    /// immediately after the drain, so drain order is the order counters are allocated in. The
    /// counter is cleartext in `OuterHeader`, so a sequence that disagrees with send order lets an
    /// observer read each packet's mixing delay straight off the wire.
    #[test]
    fn frames_are_queued_in_scheduled_order() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        // produced late-to-early, as concurrent workers drawing independent delays would
        for (seq, delay) in [(3u8, 300u64), (1, 100), (4, 400), (2, 200)] {
            outgoing.queue(frame(seq, now + Duration::from_millis(delay)));
        }

        let due = outgoing.take_due(peer(1), now + Duration::from_millis(500));
        assert_eq!(seq_of(&due), vec![1, 2, 3, 4]);
    }

    /// Nodes and clients are separate peers even when they would share a wire address.
    ///
    /// The queue is keyed by identity, so a client behind the same address as a node - or two
    /// clients behind one NAT - never share a session's counter space.
    #[test]
    fn nodes_and_clients_are_separate_peers() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        outgoing.queue(frame_for(peer(1), 1, now));
        outgoing.queue(frame_for(client_peer(1), 2, now));
        outgoing.queue(frame_for(client_peer(2), 3, now));

        assert_eq!(outgoing.peers().len(), 3);
        assert_eq!(seq_of(&outgoing.take_due(peer(1), now)), vec![1]);
        assert_eq!(seq_of(&outgoing.take_due(client_peer(1), now)), vec![2]);
        assert_eq!(seq_of(&outgoing.take_due(client_peer(2), now)), vec![3]);
    }

    /// Each peer has its own release order; the counter space is per session.
    #[test]
    fn peers_are_ordered_independently() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        outgoing.queue(frame_for(peer(1), 1, now + Duration::from_millis(300)));
        outgoing.queue(frame_for(peer(2), 2, now + Duration::from_millis(100)));
        outgoing.queue(frame_for(peer(1), 3, now + Duration::from_millis(200)));
        outgoing.queue(frame_for(peer(2), 4, now + Duration::from_millis(400)));

        let at = now + Duration::from_millis(500);
        assert_eq!(seq_of(&outgoing.take_due(peer(1), at)), vec![3, 1]);
        assert_eq!(seq_of(&outgoing.take_due(peer(2), at)), vec![2, 4]);
    }

    #[test]
    fn not_yet_due_frames_are_left_queued() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        outgoing.queue(frame(1, now));
        outgoing.queue(frame(2, now + Duration::from_millis(100)));

        assert_eq!(seq_of(&outgoing.take_due(peer(1), now)), vec![1]);
        assert!(outgoing.has_due(peer(1), now + Duration::from_millis(100)));

        let later = outgoing.take_due(peer(1), now + Duration::from_millis(100));
        assert_eq!(seq_of(&later), vec![2]);
    }

    /// A due frame whose peer has no session stays queued, and leaves once one exists.
    #[test]
    fn a_stalled_frame_waits_then_leaves() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();
        outgoing.queue(frame(1, now));

        // no session, so the pass only expires - and nothing has waited long enough yet
        assert_eq!(outgoing.drop_stalled(peer(1), now, TIMEOUT), 0);
        assert!(outgoing.has_due(peer(1), now), "held, not dropped");

        // the session appears within the timeout, so the ordinary drain takes it
        let due = outgoing.take_due(peer(1), now + TIMEOUT / 2);
        assert_eq!(seq_of(&due), vec![1]);
    }

    /// Past the timeout the frame is discarded rather than sent late.
    #[test]
    fn a_stalled_frame_is_dropped_past_the_timeout() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();
        outgoing.queue(frame(1, now));

        // pinned on both sides, so the boundary cannot drift unnoticed
        assert_eq!(outgoing.drop_stalled(peer(1), now + TIMEOUT, TIMEOUT), 0);
        assert!(outgoing.has_due(peer(1), now), "still inside the timeout");

        let past = now + TIMEOUT + Duration::from_millis(1);
        assert_eq!(outgoing.drop_stalled(peer(1), past, TIMEOUT), 1);
        assert!(!outgoing.has_due(peer(1), past));
    }

    /// Only the frames that have actually expired go.
    #[test]
    fn dropping_stalled_frames_spares_the_rest() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        outgoing.queue(frame(1, now));
        outgoing.queue(frame(2, now + TIMEOUT));

        let past = now + TIMEOUT + Duration::from_millis(1);
        assert_eq!(outgoing.drop_stalled(peer(1), past, TIMEOUT), 1);
        assert_eq!(seq_of(&outgoing.take_due(peer(1), past)), vec![2]);
    }

    /// A full queue rejects the arrival and keeps everything already scheduled.
    #[test]
    fn overflow_drops_the_arriving_frame() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        for seq in 0..MAX_QUEUED_PER_PEER {
            let at = now + Duration::from_millis(seq as u64);
            assert!(
                !outgoing.queue(frame(seq as u8, at)),
                "nothing should be dropped below the ceiling"
            );
        }

        // rejected wherever it would have sorted - furthest out, and about to go out
        assert!(outgoing.queue(frame(0xFF, now + Duration::from_secs(60))));
        assert!(outgoing.queue(frame(0xFE, now)));

        let all = outgoing.take_due(peer(1), now + Duration::from_secs(120));
        assert_eq!(all.len(), MAX_QUEUED_PER_PEER);
        assert!(!seq_of(&all).contains(&0xFF));
        assert!(!seq_of(&all).contains(&0xFE));
    }

    #[test]
    fn empty_peers_are_pruned() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        outgoing.queue(frame_for(peer(1), 1, now));
        outgoing.queue(frame_for(peer(2), 2, now + Duration::from_secs(60)));

        outgoing.take_due(peer(1), now);
        outgoing.prune_empty();

        assert_eq!(outgoing.peers(), vec![peer(2)]);
    }
}
