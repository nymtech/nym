// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::fragmentation::fragment::{Fragment, FragmentHashKey};
use crate::packet::{LpFrame, MalformedLpPacketError};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use std::fmt::Debug;
use std::ops::Add;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, trace, warn};

pub const DEFAULT_FRAGMENT_TIMEOUT_DURATION: Duration = Duration::from_secs(30);

/// Per-message buffer that collects every `Fragment` of a fragmented message
/// and reassembles the original payload once they are all in.
#[derive(Debug, Clone)]
struct MessageBuffer<Ts>
where
    Ts: PartialOrd + Clone + Debug,
{
    /// Cached completion flag, set as soon as the last missing slot has been
    /// filled. Avoids re-scanning `fragments` on every read.
    is_complete: bool,

    /// Position-indexed slots for the message's fragments. Allocated up front
    /// to `total_fragments` `None` entries on first sight of the message,
    /// giving O(1) inserts and O(n) reassembly while preserving order.
    fragments: Vec<Option<Fragment>>,

    /// Timestamp of the most recently inserted fragment. Read by
    /// [`MessageReconstructor::cleanup_stale_buffers`] to evict messages whose
    /// remaining fragments never showed up.
    last_fragment_timestamp: Ts,
}

impl<Ts> MessageBuffer<Ts>
where
    Ts: PartialOrd + Clone + Debug,
{
    /// Create an empty buffer sized for `total_fragments` slots.
    /// The `u8` argument bounds the allocation at `u8::MAX`.
    fn new(total_fragments: u8, timestamp: Ts) -> Self {
        // `new` should never be called with size 0: `total_fragments` is taken
        // from the first received `Fragment` of the message, and decoding
        // rejects any header where `current_fragment >= total_fragments`, so
        // the smallest valid value is 1.
        debug_assert!(total_fragments > 0);

        MessageBuffer {
            is_complete: false,
            fragments: vec![None; total_fragments as usize],
            last_fragment_timestamp: timestamp,
        }
    }

    /// Consume the buffer and concatenate every fragment payload into the
    /// original message bytes. The caller is expected to have observed
    /// `is_complete == true` first.
    fn into_message(self) -> Vec<u8> {
        debug_assert!(self.is_complete);

        // SAFETY: `is_complete` is only set inside `insert_fragment` after
        // `is_done_receiving` confirms every slot is `Some`. The
        // `debug_assert!` above pins this invariant, so reading slot 0 and
        // unwrapping every slot below cannot panic.
        #[allow(clippy::unwrap_used)]
        let id = self.fragments[0].as_ref().unwrap().id();
        debug!(
            "Got {} fragments for message id {}",
            self.fragments.len(),
            id
        );

        // SAFETY: same invariant as above — every slot is `Some`.
        #[allow(clippy::unwrap_used)]
        self.fragments
            .into_iter()
            .flat_map(|fragment| fragment.unwrap().extract_payload())
            .collect()
    }

    /// Whether every fragment slot has been filled.
    fn is_done_receiving(&self) -> bool {
        !self.fragments.contains(&None)
    }

    /// Insert `fragment` into the slot at `fragment.current_fragment()` and
    /// update `last_fragment_timestamp` and `is_complete` accordingly.
    ///
    /// Duplicate fragments are logged, then ignored
    fn insert_fragment(&mut self, fragment: Fragment, timestamp: Ts) {
        self.last_fragment_timestamp = timestamp;

        // All fragments routed into a given buffer must share the same id —
        // it is part of the buffer's lookup key, so a mismatch would
        // indicate a routing bug upstream.
        debug_assert!({
            let present = self.fragments.iter().find(|frag| frag.is_some());
            // SAFETY: `find` returned a slot that satisfied `is_some`, so
            // the inner `unwrap` cannot panic.
            #[allow(clippy::unwrap_used)]
            let same_id = present.is_none_or(|p| p.as_ref().unwrap().id() == fragment.id());
            same_id
        });

        let fragment_index = fragment.current_fragment() as usize;
        if self.fragments[fragment_index].is_some() {
            // If we receive a duplicate, we ignore it
            warn!(
                "duplicate fragment received! - frag - {} (message id: {})",
                fragment.current_fragment(),
                fragment.id()
            );
        } else {
            self.fragments[fragment_index] = Some(fragment);
            if self.is_done_receiving() {
                self.is_complete = true;
            }
        }
    }
}

/// Public reassembly state for fragmented messages. Buffers in-flight
/// messages keyed on their [`FragmentHashKey`] and yields the original bytes
/// once every fragment of a given message has been received.
#[derive(Debug, Clone)]
pub struct MessageReconstructor<Ts, To>
where
    Ts: PartialOrd + Debug + Clone + Add<To, Output = Ts>,
    To: Clone + Debug,
{
    /// In-flight messages keyed on `(id, frame_kind)`. The frame kind is
    /// part of the key so that a random-id collision between two unrelated
    /// kinds cannot accidentally route fragments into the same buffer.
    in_flight_messages: Arc<DashMap<FragmentHashKey, MessageBuffer<Ts>>>,

    /// How long an incomplete message is allowed to sit before it is
    /// dropped on the next `cleanup_stale_buffers` pass.
    incomplete_message_timeout: To,
}

impl<Ts, To> MessageReconstructor<Ts, To>
where
    Ts: PartialOrd + Debug + Clone + Add<To, Output = Ts>,
    To: Clone + Debug,
{
    /// Create an empty `MessageReconstructor`.
    pub fn new(incomplete_message_timeout: To) -> Self {
        Self {
            in_flight_messages: Default::default(),
            incomplete_message_timeout,
        }
    }

    /// Insert `fragment` into the buffer for its message and, if it was the
    /// last outstanding fragment, return the reassembled LpFrame
    ///
    /// Stale incomplete messages are evicted on every call.
    pub fn insert_new_fragment(
        &self,
        fragment: Fragment,
        timestamp: Ts,
    ) -> Option<Result<LpFrame, MalformedLpPacketError>> {
        let key = fragment.hash_key();
        let total_fragments = fragment.total_fragments();

        let maybe_message = match self.in_flight_messages.entry(key) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().insert_fragment(fragment, timestamp.clone());
                entry
                    .get()
                    .is_complete
                    .then(|| LpFrame::decode(&entry.remove().into_message()))
            }
            Entry::Vacant(entry) => {
                let mut buf = MessageBuffer::new(total_fragments, timestamp.clone());
                buf.insert_fragment(fragment, timestamp.clone());
                if buf.is_complete {
                    Some(LpFrame::decode(&buf.into_message()))
                } else {
                    entry.insert(buf);
                    None
                }
            }
        };

        // This might be a bit slow, keep an eye on it
        self.cleanup_stale_buffers(timestamp.clone());
        maybe_message
    }

    /// Drop incomplete messages whose `last_fragment_timestamp` is older
    /// than `incomplete_message_timeout` ago.
    pub fn cleanup_stale_buffers(&self, timestamp: Ts) {
        trace!("Cleaning up stale buffers");
        self.in_flight_messages.retain(|_, buf| {
            let keep = buf.last_fragment_timestamp.clone()
                + self.incomplete_message_timeout.clone()
                > timestamp;
            if !keep {
                debug!(
                    "Removing stale buffer for message id {:?}",
                    buf.fragments
                        .first()
                        .and_then(|f| f.as_ref().map(|f| f.id()))
                );
            }
            keep
        });
    }
}

impl Default for MessageReconstructor<Instant, Duration> {
    fn default() -> Self {
        MessageReconstructor::new(DEFAULT_FRAGMENT_TIMEOUT_DURATION)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::fragmentation::fragment::fragment_lp_message;
    use crate::packet::LpFrame;
    use crate::packet::frame::LpFrameKind;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const SPHINX: LpFrameKind = LpFrameKind::SphinxPacket;
    const OUTFOX: LpFrameKind = LpFrameKind::OutfoxPacket;

    /// Build a `Fragment` with explicit header values via the public
    /// `LpFrame` round-trip, so tests can craft duplicates, out-of-order
    /// inserts and id collisions without depending on RNG output.
    fn make_fragment(
        id: u64,
        total_fragments: u8,
        current_fragment: u8,
        inner_kind: LpFrameKind,
        payload: Vec<u8>,
    ) -> Fragment {
        let mut attrs = [0u8; 14];
        attrs[0..8].copy_from_slice(&id.to_be_bytes());
        attrs[8] = total_fragments;
        attrs[9] = current_fragment;
        attrs[10..12].copy_from_slice(&u16::to_be_bytes(inner_kind.into()));
        let frame = LpFrame::new_with_attributes(LpFrameKind::FragmentedData, attrs, payload);
        Fragment::try_from(frame).unwrap()
    }

    fn split(message: LpFrame, fragment_size: usize) -> Vec<Fragment> {
        let mut rng = StdRng::seed_from_u64(0xdead_beef);
        fragment_lp_message(&mut rng, message, fragment_size)
    }

    // ---------- MessageBuffer ----------

    #[test]
    fn buffer_completes_on_single_fragment() {
        let f = make_fragment(1, 1, 0, SPHINX, b"hi".to_vec());
        let mut buf = MessageBuffer::<u64>::new(1, 0);
        assert!(!buf.is_complete);
        buf.insert_fragment(f, 0);
        assert!(buf.is_complete);
        assert_eq!(buf.into_message(), b"hi");
    }

    #[test]
    fn buffer_completes_only_after_last_fragment() {
        let mut buf = MessageBuffer::<u64>::new(3, 0);
        buf.insert_fragment(make_fragment(7, 3, 0, SPHINX, vec![0xaa]), 1);
        assert!(!buf.is_complete);
        buf.insert_fragment(make_fragment(7, 3, 1, SPHINX, vec![0xbb]), 2);
        assert!(!buf.is_complete);
        buf.insert_fragment(make_fragment(7, 3, 2, SPHINX, vec![0xcc]), 3);
        assert!(buf.is_complete);
        assert_eq!(buf.into_message(), vec![0xaa, 0xbb, 0xcc]);
    }

    #[test]
    fn buffer_reassembles_in_order_regardless_of_insertion_order() {
        let mut buf = MessageBuffer::<u64>::new(4, 0);
        buf.insert_fragment(make_fragment(1, 4, 2, SPHINX, vec![3]), 0);
        buf.insert_fragment(make_fragment(1, 4, 0, SPHINX, vec![1]), 0);
        buf.insert_fragment(make_fragment(1, 4, 3, SPHINX, vec![4]), 0);
        buf.insert_fragment(make_fragment(1, 4, 1, SPHINX, vec![2]), 0);
        assert!(buf.is_complete);
        assert_eq!(buf.into_message(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn buffer_tracks_last_fragment_timestamp() {
        let mut buf = MessageBuffer::<u64>::new(2, 100);
        assert_eq!(buf.last_fragment_timestamp, 100);
        buf.insert_fragment(make_fragment(1, 2, 0, SPHINX, vec![0]), 250);
        assert_eq!(buf.last_fragment_timestamp, 250);
        buf.insert_fragment(make_fragment(1, 2, 1, SPHINX, vec![1]), 400);
        assert_eq!(buf.last_fragment_timestamp, 400);
    }

    #[test]
    fn buffer_duplicate_fragment_does_not_break_completion() {
        let mut buf = MessageBuffer::<u64>::new(2, 0);
        buf.insert_fragment(make_fragment(1, 2, 0, SPHINX, vec![0xaa]), 0);
        // Same slot twice
        buf.insert_fragment(make_fragment(1, 2, 0, SPHINX, vec![0xaa]), 0);
        assert!(!buf.is_complete);
        buf.insert_fragment(make_fragment(1, 2, 1, SPHINX, vec![0xbb]), 0);
        assert!(buf.is_complete);
        assert_eq!(buf.into_message(), vec![0xaa, 0xbb]);
    }

    #[test]
    fn buffer_empty_payloads_reassemble_to_empty_message() {
        let mut buf = MessageBuffer::<u64>::new(2, 0);
        buf.insert_fragment(make_fragment(1, 2, 0, SPHINX, vec![]), 0);
        buf.insert_fragment(make_fragment(1, 2, 1, SPHINX, vec![]), 0);
        assert!(buf.is_complete);
        assert!(buf.into_message().is_empty());
    }

    // ---------- MessageReconstructor: round trip via fragment_payload ----------

    #[test]
    fn reconstructor_round_trip_single_fragment_message() {
        let message = LpFrame::new(SPHINX, b"small".as_slice());
        let mut fragments = split(message.clone(), 64);
        assert_eq!(fragments.len(), 1);

        let rec = MessageReconstructor::<u64, u64>::new(60);
        let out = rec.insert_new_fragment(fragments.pop().unwrap(), 0);
        let recovered_frame = out
            .expect("single fragment must complete the message")
            .unwrap();
        assert_eq!(recovered_frame, message);
    }

    #[test]
    fn reconstructor_round_trip_multi_fragment_message() {
        let message = LpFrame::new(SPHINX, (0u8..=200).collect::<Vec<_>>());
        let fragments = split(message.clone(), 16);
        assert!(fragments.len() > 1);

        let rec = MessageReconstructor::<u64, u64>::new(60);
        let total = fragments.len();
        let mut out = None;
        for (i, f) in fragments.into_iter().enumerate() {
            out = rec.insert_new_fragment(f, i as u64);
            if i + 1 < total {
                assert!(out.is_none(), "premature completion at fragment {i}");
            }
        }
        let recovered_frame = out
            .expect("last fragment must complete the message")
            .unwrap();
        assert_eq!(recovered_frame, message);
    }

    #[test]
    fn reconstructor_handles_out_of_order_arrival() {
        let message = LpFrame::new(SPHINX, (0u8..=200).collect::<Vec<_>>());
        let mut fragments = split(message.clone(), 18);
        // Reverse arrival order.
        fragments.reverse();

        let rec = MessageReconstructor::<u64, u64>::new(60);
        let mut out = None;
        for (i, f) in fragments.into_iter().enumerate() {
            out = rec.insert_new_fragment(f, i as u64);
        }
        let recovered_frame = out
            .expect("last fragment must complete the message")
            .unwrap();
        assert_eq!(recovered_frame, message);
    }

    #[test]
    fn reconstructor_keeps_distinct_messages_separate() {
        // Two messages with different ids interleaved.
        let mut a = vec![
            make_fragment(1, 2, 0, SPHINX, vec![0xa1]),
            make_fragment(1, 2, 1, SPHINX, vec![0xa2]),
        ];
        let mut b = vec![
            make_fragment(2, 2, 0, SPHINX, vec![0xb1]),
            make_fragment(2, 2, 1, SPHINX, vec![0xb2]),
        ];

        let rec = MessageReconstructor::<u64, u64>::new(60);
        // Interleave.
        assert!(rec.insert_new_fragment(a.remove(0), 0).is_none());
        assert!(rec.insert_new_fragment(b.remove(0), 1).is_none());
        let msg_a = rec.insert_new_fragment(a.remove(0), 2).unwrap().unwrap();
        let msg_b = rec.insert_new_fragment(b.remove(0), 3).unwrap().unwrap();

        assert_eq!(msg_a.content, vec![0xa1, 0xa2]);
        assert_eq!(msg_b.content, vec![0xb1, 0xb2]);
    }

    #[test]
    fn reconstructor_does_not_mix_same_id_across_frame_kinds() {
        // Same id, different frame kinds — must not share a buffer.
        let s1 = make_fragment(42, 2, 0, SPHINX, vec![0x10]);
        let s2 = make_fragment(42, 2, 1, SPHINX, vec![0x11]);
        let o1 = make_fragment(42, 2, 0, OUTFOX, vec![0x20]);
        let o2 = make_fragment(42, 2, 1, OUTFOX, vec![0x21]);

        let rec = MessageReconstructor::<u64, u64>::new(60);
        assert!(rec.insert_new_fragment(s1, 0).is_none());
        assert!(rec.insert_new_fragment(o1, 1).is_none());
        let s_msg = rec.insert_new_fragment(s2, 2).unwrap().unwrap();
        let o_msg = rec.insert_new_fragment(o2, 3).unwrap().unwrap();

        assert_eq!(s_msg.content, vec![0x10, 0x11]);
        assert_eq!(s_msg.kind(), SPHINX);
        assert_eq!(o_msg.content, vec![0x20, 0x21]);
        assert_eq!(o_msg.kind(), OUTFOX);
    }

    #[test]
    fn reconstructor_clears_buffer_after_emitting_message() {
        let f = make_fragment(99, 1, 0, SPHINX, vec![0xff]);
        let rec = MessageReconstructor::<u64, u64>::new(60);
        let _ = rec.insert_new_fragment(f, 0).unwrap();
        assert!(
            rec.in_flight_messages.is_empty(),
            "completed messages must not linger in the in-flight map"
        );
    }

    // ---------- cleanup_stale_buffers ----------

    #[test]
    fn cleanup_evicts_buffers_older_than_timeout() {
        let f = make_fragment(1, 2, 0, SPHINX, vec![0]);
        let rec = MessageReconstructor::<u64, u64>::new(10);
        // First (and only) fragment received at t=0; the message stays
        // incomplete.
        assert!(rec.insert_new_fragment(f, 0).is_none());
        assert_eq!(rec.in_flight_messages.len(), 1);

        // Within the timeout window — buffer must survive.
        rec.cleanup_stale_buffers(5);
        assert_eq!(rec.in_flight_messages.len(), 1);

        // Past the window — evicted.
        rec.cleanup_stale_buffers(100);
        assert!(rec.in_flight_messages.is_empty());
    }

    #[test]
    fn cleanup_runs_implicitly_on_insert() {
        // Stale message at t=0, then a brand new message arrives well past
        // the timeout. The implicit cleanup inside `insert_new_fragment`
        // must drop the stale entry.
        let stale = make_fragment(1, 2, 0, SPHINX, vec![0]);
        let fresh = make_fragment(2, 1, 0, SPHINX, vec![0xff]);

        let rec = MessageReconstructor::<u64, u64>::new(10);
        assert!(rec.insert_new_fragment(stale, 0).is_none());
        assert_eq!(rec.in_flight_messages.len(), 1);

        let msg = rec.insert_new_fragment(fresh, 1_000).unwrap().unwrap();
        assert_eq!(msg.content, vec![0xff]);
        // `fresh` was a single-fragment message and is removed on emission;
        // the stale buffer must also be gone.
        assert!(rec.in_flight_messages.is_empty());
    }

    #[test]
    fn cleanup_resets_idle_timer_on_each_fragment() {
        // A buffer that keeps receiving fragments must not be evicted
        // even if the absolute time exceeds the timeout, as long as the
        // gap between fragments stays under it.
        let rec = MessageReconstructor::<u64, u64>::new(10);

        assert!(
            rec.insert_new_fragment(make_fragment(1, 3, 0, SPHINX, vec![0xa]), 0)
                .is_none()
        );
        assert!(
            rec.insert_new_fragment(make_fragment(1, 3, 1, SPHINX, vec![0xb]), 8)
                .is_none()
        );
        // Absolute time is now 16 (> 10), but the gap from the previous
        // fragment (8) to now (16) is 8, still within the 10-tick timeout.
        let out = rec.insert_new_fragment(make_fragment(1, 3, 2, SPHINX, vec![0xc]), 16);
        let msg = out.expect("buffer must still be alive").unwrap();
        assert_eq!(msg.content, vec![0xa, 0xb, 0xc]);
    }
}
