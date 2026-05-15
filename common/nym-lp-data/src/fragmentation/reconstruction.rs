// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// SW silencing clippy while devving
#![allow(clippy::unwrap_used)]

use tracing::{debug, trace, warn};

use crate::fragmentation::fragment::Fragment;
use crate::packet::frame::LpFrameKind;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Add;

/// `ReconstructionBuffer` is a per data set structure used to reconstruct the underlying data
/// and allows for relatively easy way of determining if the original message is split
/// into multiple buffers.
#[derive(Debug, Clone)]
struct ReconstructionBuffer<Ts>
where
    Ts: PartialOrd + Clone + Debug,
{
    /// Easier way to determine if buffer has received all fragments it expected to get.
    /// This way it is not required to iterate through the entire `fragments` vector looking for
    /// possible `None` elements.
    is_complete: bool,

    /// The actual `Fragment` data held by the `ReconstructionBuffer`. When created it is already
    /// appropriately resized and all missing fragments are set to a `None`, thus keeping
    /// everything in order the whole time, allowing for O(1) insertions and O(n) reconstruction.
    fragments: Vec<Option<Fragment>>,

    /// The timestamp of the last received fragment. Used for cleaning up stale buffers.
    last_fragment_timestamp: Ts,
}

impl<Ts> ReconstructionBuffer<Ts>
where
    Ts: PartialOrd + Clone + Debug,
{
    /// Initialises new instance of a `ReconstructionBuffer` with given size, i.e.
    /// number of expected `Fragment`s in the set.
    /// The `u8` input type of `size` argument ensures it has the `u8::MAX` upper bound.
    fn new(size: u8, timestamp: Ts) -> Self {
        // Note: `new` should have never been called with size 0 in the first place
        // as `size` value is based on the first recovered `Fragment` in the set.
        // A `Fragment` cannot be successfully recovered if it indicates that `total_fragments`
        // count is 0.
        debug_assert!(size > 0);

        let fragments_buffer = vec![None; size as usize];

        ReconstructionBuffer {
            is_complete: false,
            fragments: fragments_buffer,
            last_fragment_timestamp: timestamp,
        }
    }

    /// After receiving all data, consumes `self` in order to recover original data
    /// encapsulated in this particular set.
    fn reconstruct_set_data(self) -> Vec<u8> {
        // Note: `reconstruct_set_data` is never called without first explicitly checking
        // if the set is complete.
        debug_assert!(self.is_complete);

        debug!(
            "Got {} fragments for set id {}",
            self.fragments.len(),
            self.fragments[0].as_ref().unwrap().id()
        );

        self.fragments
            .into_iter()
            .map(|fragment| fragment.unwrap().extract_payload())
            .flat_map(|fragment_data| fragment_data.into_iter())
            .collect()
    }

    // TODO: check what's the performance impact of this, and if it's too big, keep track of number
    // of received fragments instead rather than checking whole vector, but then
    // we might have false positives if somehow we receive a duplicate
    /// Checks if `self` is done receiving `Fragment` data by checking if there are still
    /// any `None` elements in the `fragments` vector.
    fn is_done_receiving(&self) -> bool {
        !self.fragments.contains(&None)
    }

    /// Inserts new `Fragment` data into an appropriate position in the buffer.
    ///
    /// (Note: currently there is no defined behaviour for dealing with duplicate
    /// fragments for the same position in the set. This might potentially corrupt
    /// entire message until resolved)
    ///
    /// After new `Fragment` is inserted, it is checked whether the buffer should be
    /// done receiving and if so, the auxiliary data fields, i.e. `is_complete`,
    fn insert_fragment(&mut self, fragment: Fragment, timestamp: Ts) {
        self.last_fragment_timestamp = timestamp;

        // all fragments in the buffer should always have the same id as before inserting an element,
        // the correct buffer instance is looked up based on the fragment to be inserted.
        debug_assert!({
            let present_fragment = self.fragments.iter().find(|frag| frag.is_some());
            if let Some(existing_present_fragment) = present_fragment {
                existing_present_fragment.as_ref().unwrap().id() == fragment.id()
            } else {
                true
            }
        });

        let fragment_index = fragment.current_fragment() as usize;
        if self.fragments[fragment_index].is_some() {
            // TODO: what to do in that case? give up on the message? overwrite it? panic?
            // it *might* be due to lock ack-packet, but let's keep the `warn` level in case
            // it could be somehow exploited
            warn!(
                "duplicate fragment received! - frag - {} (set id: {})",
                fragment.current_fragment(),
                fragment.id()
            );
        }
        self.fragments[fragment_index] = Some(fragment);
        if self.is_done_receiving() {
            self.is_complete = true;
        }
    }
}

/// High level public structure used to buffer all received data `Fragment`s and eventually
/// returning original messages that they encapsulate.
#[derive(Debug, Clone)]
pub struct MessageReconstructor<Ts, To>
where
    Ts: PartialOrd + Debug + Clone + Add<To, Output = Ts>,
    To: Clone + Debug,
{
    // TODO: some cleaner thread/routine that if message is incomplete and
    // we haven't received any fragments in X time, we assume they
    // were lost and message can't be restored.
    // Perhaps add 'latest_fragment_timestamp' to each buffer
    // and after N fragments received globally, check all of buffer timestamps.
    // otherwise we are vulnerable to heap overflow attacks -> somebody can keep on sending
    // maximum sized sets but without one of required fragments. All of the received
    // data will be kept on the heap indefinitely in the current implementation.
    reconstructed_sets: HashMap<u64, ReconstructionBuffer<Ts>>,

    incomplete_message_timeout: To,
}

impl<Ts, To> MessageReconstructor<Ts, To>
where
    Ts: PartialOrd + Debug + Clone + Add<To, Output = Ts>,
    To: Clone + Debug,
{
    /// Creates an empty `MessageReconstructor`.
    pub fn new(incomplete_message_timeout: To) -> Self {
        Self {
            reconstructed_sets: Default::default(),
            incomplete_message_timeout,
        }
    }

    /// Check if set of given `id` is present in the `MessageReconstructor`, and if so,
    /// whether it has received all `Fragment`s it expected to get.
    fn is_set_fully_received(&self, id: u64) -> bool {
        self.reconstructed_sets
            .get(&id)
            .map(|set_buf| set_buf.is_complete)
            .unwrap_or_else(|| false)
    }

    /// Given id of *any* one of the sets into which message was divided,
    /// reconstruct the entire original message.
    /// Note, before you call this method, you *must* ensure all sets were fully received
    fn reconstruct_set(&mut self, set_id: u64) -> Vec<u8> {
        debug_assert!(self.is_set_fully_received(set_id));
        self.reconstructed_sets
            .remove(&set_id)
            .unwrap()
            .reconstruct_set_data()
    }

    /// Given recovered `Fragment`, tries to insert it into an appropriate `ReconstructionBuffer`.
    /// If a buffer does not exist, a new instance is created.
    /// If it was last remaining `Fragment` for the original message, the message is reconstructed
    /// and returned alongside all (if applicable) set ids used in the message.
    pub fn insert_new_fragment(
        &mut self,
        fragment: Fragment,
        timestamp: Ts,
    ) -> Option<(Vec<u8>, LpFrameKind)> {
        let set_id = fragment.id();
        let set_len = fragment.total_fragments();
        let set_kind = fragment.frame_kind();

        let buf = self
            .reconstructed_sets
            .entry(set_id)
            .or_insert_with(|| ReconstructionBuffer::new(set_len, timestamp.clone()));

        buf.insert_fragment(fragment, timestamp.clone());
        let maybe_set = if self.is_set_fully_received(set_id) {
            Some((self.reconstruct_set(set_id), set_kind))
        } else {
            None
        };
        // Cleanup stale data
        self.cleanup_stale_buffers(timestamp.clone());
        maybe_set
    }

    pub fn cleanup_stale_buffers(&mut self, timestamp: Ts) {
        trace!("Cleaning up stale buffers");
        self.reconstructed_sets.retain(|_, set_buf| {
            let keep = set_buf.last_fragment_timestamp.clone()
                + self.incomplete_message_timeout.clone()
                > timestamp;
            if !keep {
                debug!(
                    "Removing stale buffer for set id {:?}",
                    set_buf
                        .fragments
                        .first()
                        .and_then(|f| f.as_ref().map(|f| f.id()))
                );
            }
            keep
        });
    }
}
