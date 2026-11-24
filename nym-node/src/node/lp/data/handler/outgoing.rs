// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Frames waiting for their scheduled send time, and the rules for when they leave or are dropped.

use futures::StreamExt;
use nym_lp_data::packet::LpFrame;
use nym_lp_data::{AddressedTimedData, TimedData};
use nym_nonexhaustive_delayqueue::NonExhaustiveDelayQueue;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

type OutgoingFrame = AddressedTimedData<LpFrame, NymNodeRoutingAddress>;

/// Per-peer ceiling on frames waiting for a session, matching the legacy mixnet client's
/// `maximum_connection_buffer_size` - the buffer that holds packets for a peer whose connection is
/// still coming up, already tuned in production.
const MAX_WAITING_PER_PEER: usize = 192;

/// Frames awaiting their scheduled send time, and those whose time has come but whose peer has no
/// session to send them on yet.
///
/// A peer is an *identity*, not a place: a node by its address, a client by its
/// [`ClientAddress`](nym_sphinx_addressing::ClientAddress). That is what selects the session
/// encrypting the frame, and it is stable while a client's socket address is not - the wire
/// address is looked up only when the frame is finally wrapped.
///
/// Held un-encrypted: the LP counter is assigned by the transport wrap and is cleartext on the
/// wire, so it must be allocated in transmission order. Frames leave the delay queue in release
/// order and are wrapped as they leave, so the counter follows send order.
///
/// Mirrors the legacy forwarder: the delay itself is unbounded, as its producers already are
/// bounded (the worker pool and its output channel), and the per-peer bound applies only to frames
/// held back for a session, as legacy bounds a connection's buffer while it comes up.
#[derive(Default)]
pub(crate) struct OutgoingFrames {
    /// Every frame still waiting out its mix delay, across all peers.
    delayed: NonExhaustiveDelayQueue<OutgoingFrame>,

    /// Frames that are due but whose peer has no session yet, per peer in release order.
    ///
    /// Not addressed: the key already says where they go. [`Self::take_waiting`] re-attaches it.
    waiting: HashMap<NymNodeRoutingAddress, VecDeque<TimedData<LpFrame>>>,
}

impl OutgoingFrames {
    /// Hold a frame until its scheduled send time.
    pub(crate) fn delay(&mut self, frame: OutgoingFrame) {
        let at = frame.data.timestamp;
        self.delayed.insert_at(frame, at.into());
    }

    /// The next frame whose scheduled send time has arrived, waiting while nothing is delayed.
    // `NonExhaustiveDelayQueue` never ends: it stays pending while empty instead of yielding `None`
    pub(crate) async fn next_due(&mut self) -> OutgoingFrame {
        #[expect(clippy::expect_used)]
        self.delayed
            .next()
            .await
            .expect("NonExhaustiveDelayQueue returned a None value against its spec")
            .into_inner()
    }

    /// Hold a due frame until its peer has a session.
    ///
    /// Returns whether a frame was dropped to make room. At the ceiling the most overdue frame
    /// goes rather than the arrival: every waiting frame is already late, and the oldest is the
    /// most anomalous to release and the nearest to being dropped by the stall timeout anyway.
    /// Frames come out of the delay queue in release order, so appending keeps the queue ordered
    /// and its front is always the most overdue.
    pub(crate) fn wait(&mut self, frame: OutgoingFrame) -> bool {
        let AddressedTimedData { data, dst, .. } = frame;
        let queue = self.waiting.entry(dst).or_default();

        let evicted = queue.len() >= MAX_WAITING_PER_PEER;
        if evicted {
            queue.pop_front();
        }
        queue.push_back(data);

        evicted
    }

    /// Whether any frame is waiting for a session.
    pub(crate) fn has_waiting(&self) -> bool {
        !self.waiting.is_empty()
    }

    /// Every peer with frames waiting for a session.
    ///
    /// Collected rather than borrowed so the caller can consult the session store and the dialer
    /// while acting on each peer.
    pub(crate) fn waiting_peers(&self) -> Vec<NymNodeRoutingAddress> {
        self.waiting.keys().copied().collect()
    }

    /// Take every frame waiting on `peer`, in release order, and forget the peer.
    pub(crate) fn take_waiting(&mut self, peer: NymNodeRoutingAddress) -> Vec<OutgoingFrame> {
        let Some(queue) = self.waiting.remove(&peer) else {
            return Vec::new();
        };

        queue
            .into_iter()
            .map(|frame| AddressedTimedData::new_addressed(frame.timestamp, frame.data, peer))
            .collect()
    }

    /// Discard `peer`'s waiting frames that are more than `timeout` past their send time,
    /// returning how many.
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
        let Some(queue) = self.waiting.get_mut(&peer) else {
            return 0;
        };

        let stalled =
            queue.partition_point(|frame| now.saturating_duration_since(frame.timestamp) > timeout);
        queue.drain(..stalled);

        if queue.is_empty() {
            self.waiting.remove(&peer);
        }
        stalled
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

    fn frame_for(dst: NymNodeRoutingAddress, seq: u8, at: Instant) -> OutgoingFrame {
        AddressedTimedData::new_addressed(at, LpFrame::new(LpFrameKind::Opaque, vec![seq]), dst)
    }

    fn frame(seq: u8, at: Instant) -> OutgoingFrame {
        frame_for(peer(1), seq, at)
    }

    fn seq_of(frames: &[OutgoingFrame]) -> Vec<u8> {
        frames.iter().map(|f| f.data.data.content[0]).collect()
    }

    /// Now on tokio's clock, which the delay queue runs on and which is paused in these tests.
    fn tokio_now() -> Instant {
        tokio::time::Instant::now().into_std()
    }

    async fn next_seq(outgoing: &mut OutgoingFrames) -> u8 {
        outgoing.next_due().await.data.data.content[0]
    }

    /// Frames leave in scheduled order however the workers happened to produce them, across peers.
    ///
    /// This is what keeps LP counters monotonic on the wire: the counter is assigned by the wrap as
    /// a frame leaves, so release order is the order counters are allocated in. The counter is
    /// cleartext in `OuterHeader`, so a sequence that disagrees with send order lets an observer
    /// read each packet's mixing delay straight off the wire.
    #[tokio::test(start_paused = true)]
    async fn frames_leave_in_scheduled_order() {
        let now = tokio_now();
        let mut outgoing = OutgoingFrames::default();

        // produced late-to-early, as concurrent workers drawing independent delays would
        for (dst, seq, delay) in [
            (peer(1), 3u8, 300u64),
            (peer(2), 1, 100),
            (client_peer(1), 4, 400),
            (peer(1), 2, 200),
        ] {
            outgoing.delay(frame_for(dst, seq, now + Duration::from_millis(delay)));
        }

        for expected in 1..=4 {
            assert_eq!(next_seq(&mut outgoing).await, expected);
        }
    }

    /// A frame that arrives already overdue comes out without waiting.
    #[tokio::test(start_paused = true)]
    async fn an_overdue_frame_leaves_at_once() {
        let start = tokio::time::Instant::now();
        let mut outgoing = OutgoingFrames::default();

        outgoing.delay(frame(1, tokio_now() - Duration::from_millis(5)));
        assert_eq!(next_seq(&mut outgoing).await, 1);
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    /// Nothing delayed means nothing comes out, rather than the queue ending.
    #[tokio::test(start_paused = true)]
    async fn an_empty_delay_queue_waits() {
        let mut outgoing = OutgoingFrames::default();
        let res = tokio::time::timeout(Duration::from_secs(3600), outgoing.next_due()).await;
        assert!(res.is_err());
    }

    /// Waiting frames keep their order and come back addressed, per peer.
    ///
    /// The queue is keyed by identity, so a client behind the same address as a node - or two
    /// clients behind one NAT - never share a session's counter space.
    #[test]
    fn waiting_frames_come_back_in_order_per_peer() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        outgoing.wait(frame_for(peer(1), 1, now));
        outgoing.wait(frame_for(client_peer(1), 2, now));
        outgoing.wait(frame_for(peer(1), 3, now));

        assert_eq!(outgoing.waiting_peers().len(), 2);

        let node_frames = outgoing.take_waiting(peer(1));
        assert_eq!(seq_of(&node_frames), vec![1, 3]);
        assert!(node_frames.iter().all(|f| f.dst == peer(1)));
        assert_eq!(seq_of(&outgoing.take_waiting(client_peer(1))), vec![2]);

        assert!(!outgoing.has_waiting());
        assert!(outgoing.take_waiting(peer(1)).is_empty());
    }

    /// A full waiting queue sheds its most overdue frame for the arrival, and only for that peer.
    #[test]
    fn overflow_evicts_the_most_overdue_frame() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        for seq in 0..MAX_WAITING_PER_PEER {
            let at = now + Duration::from_millis(seq as u64);
            assert!(
                !outgoing.wait(frame(seq as u8, at)),
                "nothing should be dropped below the ceiling"
            );
        }
        assert!(!outgoing.wait(frame_for(peer(2), 0xEE, now)));

        assert!(outgoing.wait(frame(0xFF, now + Duration::from_secs(1))));

        let waiting = seq_of(&outgoing.take_waiting(peer(1)));
        assert_eq!(waiting.len(), MAX_WAITING_PER_PEER);
        assert_eq!(waiting.first(), Some(&1), "the oldest went");
        assert_eq!(waiting.last(), Some(&0xFF), "the arrival stayed");
        assert_eq!(seq_of(&outgoing.take_waiting(peer(2))), vec![0xEE]);
    }

    /// A waiting frame inside the timeout is kept, and leaves once its session exists.
    #[test]
    fn a_stalled_frame_waits_then_leaves() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();
        outgoing.wait(frame(1, now));

        assert_eq!(outgoing.drop_stalled(peer(1), now, TIMEOUT), 0);
        assert!(outgoing.has_waiting(), "held, not dropped");

        assert_eq!(seq_of(&outgoing.take_waiting(peer(1))), vec![1]);
    }

    /// Past the timeout the frame is discarded rather than sent late, and the emptied peer goes.
    #[test]
    fn a_stalled_frame_is_dropped_past_the_timeout() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();
        outgoing.wait(frame(1, now));

        // pinned on both sides, so the boundary cannot drift unnoticed
        assert_eq!(outgoing.drop_stalled(peer(1), now + TIMEOUT, TIMEOUT), 0);
        assert!(outgoing.has_waiting(), "still inside the timeout");

        let past = now + TIMEOUT + Duration::from_millis(1);
        assert_eq!(outgoing.drop_stalled(peer(1), past, TIMEOUT), 1);
        assert!(!outgoing.has_waiting());
    }

    /// Only the frames that have actually expired go.
    #[test]
    fn dropping_stalled_frames_spares_the_rest() {
        let now = Instant::now();
        let mut outgoing = OutgoingFrames::default();

        outgoing.wait(frame(1, now));
        outgoing.wait(frame(2, now + TIMEOUT));

        let past = now + TIMEOUT + Duration::from_millis(1);
        assert_eq!(outgoing.drop_stalled(peer(1), past, TIMEOUT), 1);
        assert_eq!(seq_of(&outgoing.take_waiting(peer(1))), vec![2]);
    }
}
