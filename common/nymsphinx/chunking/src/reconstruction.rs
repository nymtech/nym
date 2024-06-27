// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use crate::fragment::Fragment;
use crate::ChunkingError;
use log::*;
use std::collections::HashMap;

// TODO: perhaps a more sophisticated approach with writing to disk periodically in case
// we're receiving fast & furious in uncompressed 4K - we don't want to keep that in memory;
// perhaps write whole sets to the disk if there are still more of them to recover?
// Then either combine files on the disk to target destination or read everything to memory
/// `ReconstructionBuffer` is a per data set structure used to reconstruct the underlying data
/// and allows for relatively easy way of determining if the original message is split
/// into multiple buffers.
#[derive(PartialEq, Debug, Clone)]
struct ReconstructionBuffer {
    /// Easier way to determine if buffer has received all fragments it expected to get.
    /// This way it is not required to iterate through the entire `fragments` vector looking for
    /// possible `None` elements.
    is_complete: bool,

    /// Once all fragments are received, the value of `previous_fragments_set_id` is copied
    /// from the first `Fragment` in the set.
    previous_fragments_set_id: Option<i32>,
    /// Once all fragments are received, the value of `next_fragments_set_id` is copied
    /// from the last `Fragment` in the set (assuming the set is full, i.e. it contains
    /// `u8::MAX` elements).
    next_fragments_set_id: Option<i32>,

    /// The actual `Fragment` data held by the `ReconstructionBuffer`. When created it is already
    /// appropriately resized and all missing fragments are set to a `None`, thus keeping
    /// everything in order the whole time, allowing for O(1) insertions and O(n) reconstruction.
    fragments: Vec<Option<Fragment>>,
}

/// Type alias representing fully reconstructed message - its original data and list of all
/// set ids used for the reconstructions processed so that they could be used for replay prevention.
pub type ReconstructedMessage = (Vec<u8>, Vec<i32>);

impl ReconstructionBuffer {
    /// Initialises new instance of a `ReconstructionBuffer` with given size, i.e.
    /// number of expected `Fragment`s in the set.
    /// The `u8` input type of `size` argument ensures it has the `u8::MAX` upper bound.
    fn new(size: u8) -> Self {
        // Note: `new` should have never been called with size 0 in the first place
        // as `size` value is based on the first recovered `Fragment` in the set.
        // A `Fragment` cannot be successfully recovered if it indicates that `total_fragments`
        // count is 0.
        debug_assert!(size > 0);

        let mut fragments_buffer = Vec::new();
        fragments_buffer.resize(size as usize, None);

        ReconstructionBuffer {
            is_complete: false,
            previous_fragments_set_id: None,
            next_fragments_set_id: None,
            fragments: fragments_buffer,
        }
    }

    /// After receiving all data, consumes `self` in order to recover original data
    /// encapsulated in this particular set.
    fn reconstruct_set_data(self) -> Vec<u8> {
        // Note: `reconstruct_set_data` is never called without first explicitly checking
        // if the set is complete.
        debug_assert!(self.is_complete);

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
    /// `previous_fragments_set_id` and `next_fragments_set_id` are set for the ease
    /// of access.
    fn insert_fragment(&mut self, fragment: Fragment) {
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

        let fragment_index = fragment.current_fragment() as usize - 1;
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
            self.previous_fragments_set_id = self.fragments[0]
                .as_ref()
                .unwrap()
                .previous_fragments_set_id();
            self.next_fragments_set_id = if self.fragments.len() == u8::MAX as usize {
                self.fragments[u8::MAX as usize - 1]
                    .as_ref()
                    .unwrap()
                    .next_fragments_set_id()
            } else {
                None
            };
        }
    }
}

/// High level public structure used to buffer all received data `Fragment`s and eventually
/// returning original messages that they encapsulate.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct MessageReconstructor {
    // TODO: some cleaner thread/routine that if message is incomplete and
    // we haven't received any fragments in X time, we assume they
    // were lost and message can't be restored.
    // Perhaps add 'latest_fragment_timestamp' to each buffer
    // and after N fragments received globally, check all of buffer timestamps.
    // otherwise we are vulnerable to heap overflow attacks -> somebody can keep on sending
    // maximum sized sets but without one of required fragments. All of the received
    // data will be kept on the heap indefinitely in the current implementation.
    reconstructed_sets: HashMap<i32, ReconstructionBuffer>,
}

impl MessageReconstructor {
    /// Creates an empty `MessageReconstructor`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Given fully received set of given `id`, if it has any post-linked sets, recursively
    /// checks if all of them were also fully received.
    fn check_front_chain(&self, id: i32) -> bool {
        // we know that set with `id` was fully_received (otherwise this method wouldn't have been called)
        // and hence the buffer has all of its fields properly set
        debug_assert!(self.is_set_fully_received(id));

        if let Some(previous_id) = self.previous_linked_set_id(id) {
            self.is_set_fully_received(previous_id) && self.check_front_chain(previous_id)
        } else {
            true
        }
    }

    /// Given fully received set of given `id`, if it has any pre-linked sets, recursively
    /// checks if all of them were also fully received.
    fn check_back_chain(&self, id: i32) -> bool {
        // we know that set with `id` was fully_received (otherwise this method wouldn't have been called)
        // and hence the buffer has all of its fields properly set
        debug_assert!(self.is_set_fully_received(id));

        if let Some(next_id) = self.next_linked_set_id(id) {
            self.is_set_fully_received(next_id) && self.check_back_chain(next_id)
        } else {
            true
        }
    }

    /// Check if set of given `id` is present in the `MessageReconstructor`, and if so,
    /// whether it has received all `Fragment`s it expected to get.
    fn is_set_fully_received(&self, id: i32) -> bool {
        self.reconstructed_sets
            .get(&id)
            .map(|set_buf| set_buf.is_complete)
            .unwrap_or_else(|| false)
    }

    /// Check if message that was split into possibly multiple sets was received in fully using
    /// `id` of any of its sets.
    fn is_message_fully_received(&self, id: i32) -> bool {
        self.is_set_fully_received(id) && self.check_back_chain(id) && self.check_front_chain(id)
    }

    /// Given id of *any* one of the sets into which message was split,
    /// try to obtain id of the set containing head of the message.
    /// Might return `None` if one of the sets was not fully received.
    fn find_starting_set_id(&self, id: i32) -> Option<i32> {
        if self.is_set_fully_received(id) {
            if let Some(previous_id) = self.previous_linked_set_id(id) {
                self.find_starting_set_id(previous_id)
            } else {
                Some(id)
            }
        } else {
            None
        }
    }

    /// Given id of a set, obtains (if applicable) id of the previous linked set.
    /// Note, before you call this method, you *must* ensure set was fully received
    fn previous_linked_set_id(&self, id: i32) -> Option<i32> {
        debug_assert!(self.is_set_fully_received(id));
        self.reconstructed_sets
            .get(&id)
            .unwrap()
            .previous_fragments_set_id
    }

    /// Given id of a set, obtains (if applicable) id of the next linked set.
    /// Note, before you call this method, you *must* ensure set was fully received
    fn next_linked_set_id(&self, id: i32) -> Option<i32> {
        debug_assert!(self.is_set_fully_received(id));
        self.reconstructed_sets
            .get(&id)
            .unwrap()
            .next_fragments_set_id
    }

    /// Given id of a set, consume its buffer and reconstruct the original payload.
    /// Note, before you call this method, you *must* ensure set was fully received
    fn extract_set_payload(&mut self, set_id: i32) -> Vec<u8> {
        debug_assert!(self.is_set_fully_received(set_id));
        self.reconstructed_sets
            .remove(&set_id)
            .unwrap()
            .reconstruct_set_data()
    }

    // Future consideration: perhaps for long messages, rather than return whole data allocated
    // on the heap, return file handle with the saved content?
    /// Given id of *any* one of the sets into which message was divided,
    /// reconstruct the entire original message.
    /// Note, before you call this method, you *must* ensure all sets were fully received
    fn reconstruct_message(&mut self, set_id: i32) -> ReconstructedMessage {
        debug_assert!(self.is_message_fully_received(set_id));
        let starting_id = self.find_starting_set_id(set_id).unwrap();
        let set_id_sequence: Vec<_> =
            std::iter::successors(Some(starting_id), |&id| self.next_linked_set_id(id)).collect();

        let message_content: Vec<_> = set_id_sequence
            .iter()
            .map(|&id| self.extract_set_payload(id))
            .flat_map(|payload| payload.into_iter())
            .collect();

        (message_content, set_id_sequence)
    }

    /// Given recovered `Fragment`, tries to insert it into an appropriate `ReconstructionBuffer`.
    /// If a buffer does not exist, a new instance is created.
    /// If it was last remaining `Fragment` for the original message, the message is reconstructed
    /// and returned alongside all (if applicable) set ids used in the message.
    pub fn insert_new_fragment(&mut self, fragment: Fragment) -> Option<ReconstructedMessage> {
        let set_id = fragment.id();
        let set_len = fragment.total_fragments();

        let buf = self
            .reconstructed_sets
            .entry(set_id)
            .or_insert_with(|| ReconstructionBuffer::new(set_len));

        buf.insert_fragment(fragment);
        if self.is_message_fully_received(set_id) {
            Some(self.reconstruct_message(set_id))
        } else {
            None
        }
    }

    /// Given raw `Fragment` data, tries to decode and return it.
    pub fn recover_fragment(&self, fragment_data: Vec<u8>) -> Result<Fragment, ChunkingError> {
        Fragment::try_from_bytes(&fragment_data)
    }
}

#[cfg(test)]
mod reconstruction_buffer {
    use super::*;
    use crate::fragment::unlinked_fragment_payload_max_len;
    use crate::set::max_one_way_linked_set_payload_length;

    // just some arbitrary value to use in tests
    const AVAILABLE_PLAINTEXT_SIZE: usize = 1024;

    #[test]
    fn creating_new_instance_correctly_initialised_fragments_buffer() {
        let buf = ReconstructionBuffer::new(1);
        assert_eq!(1, buf.fragments.len());
        for frag in buf.fragments {
            assert_eq!(None, frag);
        }

        let buf = ReconstructionBuffer::new(42);
        assert_eq!(42, buf.fragments.len());
        for frag in buf.fragments {
            assert_eq!(None, frag);
        }

        let buf = ReconstructionBuffer::new(u8::MAX);
        assert_eq!(u8::MAX as usize, buf.fragments.len());
        for frag in buf.fragments {
            assert_eq!(None, frag);
        }
    }

    #[test]
    #[should_panic]
    fn creating_new_instance_does_not_allow_for_creating_zero_sized_buffer() {
        ReconstructionBuffer::new(0);
    }

    #[test]
    fn reconstructing_set_data_works_for_buffers_of_different_sizes() {
        let mut buf = ReconstructionBuffer::new(1);
        let message = vec![42u8; 42];

        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        // acks are ignored as they will be stripped by gateways before getting to the reconstruction

        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[0]).unwrap());
        assert_eq!(message.to_vec(), buf.reconstruct_set_data());

        let mut buf = ReconstructionBuffer::new(3);
        let message = vec![42u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();
        assert_eq!(raw_fragments.len(), 3);

        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[0]).unwrap());
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[1]).unwrap());
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[2]).unwrap());
        assert_eq!(message.to_vec(), buf.reconstruct_set_data());

        let mut buf = ReconstructionBuffer::new(u8::MAX);
        let message = vec![
            42u8;
            unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)
                * u8::MAX as usize
        ];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();
        for raw_fragment in raw_fragments {
            buf.insert_fragment(Fragment::try_from_bytes(&raw_fragment).unwrap())
        }
        assert_eq!(message.to_vec(), buf.reconstruct_set_data());
    }

    #[test]
    #[should_panic]
    fn reconstructing_set_data_is_not_allowed_for_incomplete_sets() {
        let mut buf = ReconstructionBuffer::new(3);
        let raw_fragments: Vec<_> = crate::split_into_sets(
            &mut rand::rngs::OsRng,
            &vec![42u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3],
            AVAILABLE_PLAINTEXT_SIZE,
        )
        .into_iter()
        .flat_map(|fragment_set| fragment_set.into_iter())
        .map(|x| x.into_bytes())
        .collect();
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[0]).unwrap());
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[1]).unwrap());

        buf.reconstruct_set_data();
    }

    #[test]
    fn inserting_new_fragment_puts_it_in_correct_location_based_on_its_ordering() {
        let mut buf = ReconstructionBuffer::new(3);
        let raw_fragments: Vec<_> = crate::split_into_sets(
            &mut rand::rngs::OsRng,
            &vec![42u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3],
            AVAILABLE_PLAINTEXT_SIZE,
        )
        .into_iter()
        .flat_map(|fragment_set| fragment_set.into_iter())
        .map(|x| x.into_bytes())
        .collect();
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[1]).unwrap());

        assert!(buf.fragments[0].is_none());
        assert!(buf.fragments[1].is_some());
        assert!(buf.fragments[2].is_none());
    }

    #[test]
    fn inserting_final_fragment_correctly_sets_auxiliary_flags() {
        let mut buf = ReconstructionBuffer::new(3);
        let message = vec![42u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[0]).unwrap());
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[2]).unwrap());

        assert!(!buf.is_complete);
        assert!(buf.previous_fragments_set_id.is_none());
        assert!(buf.next_fragments_set_id.is_none());
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[1]).unwrap());
        assert!(buf.is_complete);
        assert!(buf.previous_fragments_set_id.is_none());
        assert!(buf.next_fragments_set_id.is_none());

        let mut buf = ReconstructionBuffer::new(255);
        let message =
            vec![42u8; max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE) + 123];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        for raw_fragment in raw_fragments.iter().take(u8::MAX as usize - 1) {
            buf.insert_fragment(Fragment::try_from_bytes(raw_fragment).unwrap());
        }

        assert!(!buf.is_complete);
        assert!(buf.previous_fragments_set_id.is_none());
        assert!(buf.next_fragments_set_id.is_none());
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[254]).unwrap());
        assert!(buf.is_complete);
        assert!(buf.previous_fragments_set_id.is_none());
        assert!(buf.next_fragments_set_id.is_some());

        let mut buf = ReconstructionBuffer::new(1);
        assert!(!buf.is_complete);
        assert!(buf.previous_fragments_set_id.is_none());
        assert!(buf.next_fragments_set_id.is_none());
        let fragment = Fragment::try_from_bytes(&raw_fragments[255]);
        buf.insert_fragment(fragment.unwrap());
        assert!(buf.is_complete);
        assert!(buf.previous_fragments_set_id.is_some());
        assert!(buf.next_fragments_set_id.is_none());
    }

    #[test]
    #[should_panic]
    fn does_not_allow_for_inserting_new_fragments_with_different_ids() {
        let mut buf = ReconstructionBuffer::new(3);

        // they will have different IDs
        let raw_fragments1: Vec<_> = crate::split_into_sets(
            &mut rand::rngs::OsRng,
            &vec![42u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3],
            AVAILABLE_PLAINTEXT_SIZE,
        )
        .into_iter()
        .flat_map(|fragment_set| fragment_set.into_iter())
        .map(|x| x.into_bytes())
        .collect();
        let raw_fragments2: Vec<_> = crate::split_into_sets(
            &mut rand::rngs::OsRng,
            &vec![42u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3],
            AVAILABLE_PLAINTEXT_SIZE,
        )
        .into_iter()
        .flat_map(|fragment_set| fragment_set.into_iter())
        .map(|x| x.into_bytes())
        .collect();

        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments1[0]).unwrap());
        buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments2[0]).unwrap());
    }
}

#[cfg(test)]
mod message_reconstructor {
    use super::*;
    use crate::fragment::unlinked_fragment_payload_max_len;
    use crate::set::{max_one_way_linked_set_payload_length, two_way_linked_set_payload_length};
    use rand::{thread_rng, RngCore};

    // just some arbitrary value to use in tests
    const AVAILABLE_PLAINTEXT_SIZE: usize = 1024;

    #[test]
    #[should_panic]
    fn checking_front_chain_is_not_allowed_for_incomplete_sets() {
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![
            42u8;
            max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)
                + 123
        ];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        // first set is fully inserted
        for raw_fragment in raw_fragments.iter() {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none())
        }

        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[255].clone())
                    .unwrap()
            )
            .is_none());

        let second_set_id = Fragment::try_from_bytes(&raw_fragments[255]).unwrap().id();
        reconstructor.check_front_chain(second_set_id);
    }

    #[test]
    #[should_panic]
    fn checking_back_chain_is_not_allowed_for_incomplete_sets() {
        let mut reconstructor = MessageReconstructor::default();

        let message =
            vec![42u8; max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE) + 123];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        for raw_fragment in raw_fragments.iter().take(u8::MAX as usize) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        // finish next set for good measure
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[255].clone())
                    .unwrap()
            )
            .is_none());
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[256].clone())
                    .unwrap()
            )
            .is_none());

        let first_set_id = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();
        reconstructor.check_back_chain(first_set_id);
    }

    #[test]
    fn checking_front_chain_returns_false_for_complete_set_but_incomplete_message() {
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![
            42u8;
            max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)
                + 123
        ];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        // note that first set is not fully inserted
        for raw_fragment in raw_fragments.iter().take(u8::MAX as usize - 1) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[255].clone())
                    .unwrap()
            )
            .is_none());
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[256].clone())
                    .unwrap()
            )
            .is_none());

        let second_set_id = Fragment::try_from_bytes(&raw_fragments[255]).unwrap().id();
        assert!(!reconstructor.check_front_chain(second_set_id));
    }

    #[test]
    fn checking_back_chain_returns_false_for_complete_set_but_incomplete_message() {
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![
            42u8;
            max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)
                + 123
        ];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        for raw_fragment in raw_fragments.iter().take(u8::MAX as usize) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        // notice that entirety of second set is not inserted
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[255].clone())
                    .unwrap()
            )
            .is_none());

        let first_set_id = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();

        assert!(!reconstructor.check_back_chain(first_set_id));
    }

    #[test]
    fn checking_front_chain_returns_true_for_if_there_are_no_more_front_sets() {
        // case of 2 sets: [id1 -- id2], where id1 is completed and being checked
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![
            42u8;
            max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)
                + 123
        ];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        for raw_fragment in raw_fragments.iter().take(u8::MAX as usize) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        // notice that entirety of second set is not inserted
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[255].clone())
                    .unwrap()
            )
            .is_none());

        let first_set_id = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();

        assert!(reconstructor.check_front_chain(first_set_id));
    }

    #[test]
    fn checking_back_chain_returns_true_for_if_there_are_no_more_back_sets() {
        // case of 2 sets: [id1 -- id2], where id2 is completed and being checked
        let mut reconstructor = MessageReconstructor::default();

        let message =
            vec![42u8; max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE) + 123];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        // note that first set is not fully inserted
        for raw_fragment in raw_fragments.iter().take(u8::MAX as usize - 1) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[255].clone())
                    .unwrap()
            )
            .is_none());

        let second_set_id = Fragment::try_from_bytes(&raw_fragments[255]).unwrap().id();
        assert!(reconstructor.check_back_chain(second_set_id));
    }

    #[test]
    fn checking_front_chain_returns_true_for_complete_front_chain() {
        // case of 3 sets: [id1 -- id2 -- id3], where id1 and id2 are completed and id2 is being checked
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![
            42u8;
            max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + two_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)
                + 123
        ];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        for raw_fragment in raw_fragments.iter().take(u8::MAX as usize * 2) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        // notice that entirety of third set is not inserted
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[(u8::MAX as usize) * 2].clone())
                    .unwrap()
            )
            .is_none());

        let second_set_id = Fragment::try_from_bytes(&raw_fragments[300]).unwrap().id();

        assert!(reconstructor.check_front_chain(second_set_id));
    }

    #[test]
    fn checking_back_chain_returns_true_for_complete_back_chain() {
        // case of 3 sets: [id1 -- id2 -- id3], where id2 and id3 are completed and id2 is being checked
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![
            42u8;
            max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + two_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + 123
        ];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        // note that first set is not fully inserted
        for raw_fragment in raw_fragments.iter().skip(1).take(u8::MAX as usize * 2 - 1) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[(u8::MAX as usize) * 2].clone())
                    .unwrap()
            )
            .is_none());

        let second_set_id = Fragment::try_from_bytes(&raw_fragments[300]).unwrap().id();

        assert!(reconstructor.check_back_chain(second_set_id));
    }

    #[test]
    fn checking_if_set_is_fully_received_returns_false_if_no_fragments_were_ever_received() {
        let reconstructor = MessageReconstructor::default();
        assert!(!reconstructor.is_set_fully_received(12345));
    }

    #[test]
    fn checking_if_set_is_fully_received_if_exists_returns_whatever_is_complete_flag_is_set_to() {
        let mut reconstructor = MessageReconstructor::default();
        reconstructor.reconstructed_sets.insert(
            12345,
            ReconstructionBuffer {
                is_complete: false,
                previous_fragments_set_id: None,
                next_fragments_set_id: None,
                fragments: vec![],
            },
        );

        reconstructor.reconstructed_sets.insert(
            1234,
            ReconstructionBuffer {
                is_complete: true,
                previous_fragments_set_id: None,
                next_fragments_set_id: None,
                fragments: vec![],
            },
        );

        assert!(!reconstructor.is_set_fully_received(12345));
        assert!(reconstructor.is_set_fully_received(1234));
    }

    #[test]
    fn finding_starting_set_id_returns_none_if_message_was_not_fully_received() {
        let mut reconstructor = MessageReconstructor::default();

        let message1 =
            vec![42u8; max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE) + 123];
        let raw_fragments1: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message1, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        // note that first set is not fully inserted
        for raw_fragment in raw_fragments1.iter().take(u8::MAX as usize - 1) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments1[255].clone())
                    .unwrap()
            )
            .is_none());

        let second_set_id = Fragment::try_from_bytes(&raw_fragments1[255]).unwrap().id();
        assert!(reconstructor.find_starting_set_id(second_set_id).is_none());

        let message2 = vec![
            43u8;
            max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)
                + 123
        ];
        let raw_fragments2: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message2, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        for raw_fragment in raw_fragments2.iter().take(u8::MAX as usize) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        // notice that entirety of second set is not inserted
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments2[255].clone())
                    .unwrap()
            )
            .is_none());

        let second_set_id = Fragment::try_from_bytes(&raw_fragments2[255]).unwrap().id();
        assert!(reconstructor.find_starting_set_id(second_set_id).is_none());
    }

    #[test]
    fn finding_starting_set_id_returns_expected_starting_id() {
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![
            42u8;
            max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                + unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)
                + 123
        ];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        for raw_fragment in raw_fragments.iter().take(u8::MAX as usize) {
            assert!(reconstructor
                .insert_new_fragment(
                    reconstructor
                        .recover_fragment(raw_fragment.clone())
                        .unwrap()
                )
                .is_none());
        }

        // notice that entirety of second set is not inserted
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[255].clone())
                    .unwrap()
            )
            .is_none());

        let first_set_id = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();
        assert_eq!(
            reconstructor.find_starting_set_id(first_set_id),
            Some(first_set_id)
        );

        reconstructor.reconstructed_sets.insert(
            12345,
            ReconstructionBuffer {
                is_complete: true,
                previous_fragments_set_id: None,
                next_fragments_set_id: Some(1234),
                fragments: vec![],
            },
        );

        reconstructor.reconstructed_sets.insert(
            1234,
            ReconstructionBuffer {
                is_complete: true,
                previous_fragments_set_id: Some(12345),
                next_fragments_set_id: Some(123),
                fragments: vec![],
            },
        );

        reconstructor.reconstructed_sets.insert(
            123,
            ReconstructionBuffer {
                is_complete: true,
                previous_fragments_set_id: Some(1234),
                next_fragments_set_id: Some(12),
                fragments: vec![],
            },
        );

        reconstructor.reconstructed_sets.insert(
            12,
            ReconstructionBuffer {
                is_complete: true,
                previous_fragments_set_id: Some(123),
                next_fragments_set_id: None,
                fragments: vec![],
            },
        );

        assert_eq!(reconstructor.find_starting_set_id(12), Some(12345));
    }

    #[test]
    #[should_panic]
    fn getting_previous_linked_set_id_is_not_allowed_for_incomplete_sets() {
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![42u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[0].clone())
                    .unwrap()
            )
            .is_none());
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[1].clone())
                    .unwrap()
            )
            .is_none());

        let id = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();
        reconstructor.previous_linked_set_id(id);
    }

    #[test]
    fn getting_previous_linked_set_id_returns_id_of_previous_set() {
        let mut reconstructor = MessageReconstructor::default();
        reconstructor.reconstructed_sets.insert(
            12345,
            ReconstructionBuffer {
                is_complete: true,
                previous_fragments_set_id: None,
                next_fragments_set_id: Some(1234),
                fragments: vec![],
            },
        );
        reconstructor.reconstructed_sets.insert(
            1234,
            ReconstructionBuffer {
                is_complete: true,
                previous_fragments_set_id: Some(12345),
                next_fragments_set_id: None,
                fragments: vec![],
            },
        );
        assert_eq!(reconstructor.previous_linked_set_id(12345), None);
        assert_eq!(reconstructor.previous_linked_set_id(1234), Some(12345));
    }

    #[test]
    #[should_panic]
    fn getting_next_linked_set_id_is_not_allowed_for_incomplete_sets() {
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![42u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[0].clone())
                    .unwrap()
            )
            .is_none());
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[1].clone())
                    .unwrap()
            )
            .is_none());

        let id = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();
        reconstructor.next_linked_set_id(id);
    }

    #[test]
    fn getting_next_linked_set_id_returns_id_of_next_set() {
        let mut reconstructor = MessageReconstructor::default();
        reconstructor.reconstructed_sets.insert(
            12345,
            ReconstructionBuffer {
                is_complete: true,
                previous_fragments_set_id: None,
                next_fragments_set_id: Some(1234),
                fragments: vec![],
            },
        );
        reconstructor.reconstructed_sets.insert(
            1234,
            ReconstructionBuffer {
                is_complete: true,
                previous_fragments_set_id: Some(12345),
                next_fragments_set_id: None,
                fragments: vec![],
            },
        );
        assert_eq!(reconstructor.next_linked_set_id(12345), Some(1234));
        assert_eq!(reconstructor.next_linked_set_id(1234), None);
    }

    #[test]
    #[should_panic]
    fn extracting_set_payload_is_not_allowed_for_incomplete_sets() {
        let mut reconstructor = MessageReconstructor::default();

        let message = vec![42u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3];
        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[0].clone())
                    .unwrap()
            )
            .is_none());
        assert!(reconstructor
            .insert_new_fragment(
                reconstructor
                    .recover_fragment(raw_fragments[1].clone())
                    .unwrap()
            )
            .is_none());

        let id = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();
        reconstructor.extract_set_payload(id);
    }

    #[test]
    fn extracting_set_payload_is_returns_entire_set_data() {
        let mut reconstructor = MessageReconstructor::default();
        let mut set_buf = ReconstructionBuffer::new(3);
        let mut rng = thread_rng();

        let mut message =
            vec![0u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3];
        rng.fill_bytes(&mut message);

        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();
        set_buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[0]).unwrap());
        set_buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[1]).unwrap());
        set_buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[2]).unwrap());

        let set_id = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();
        let buf_clone = set_buf.clone();
        let another_buf_clone = set_buf.clone();
        reconstructor.reconstructed_sets.insert(set_id, set_buf);
        assert_eq!(
            reconstructor.extract_set_payload(set_id),
            buf_clone.reconstruct_set_data()
        );
        assert_eq!(another_buf_clone.reconstruct_set_data(), message.to_vec());
    }

    #[test]
    fn reconstructing_message_for_single_set_is_equivalent_to_extracting_set_payload() {
        // we're inserting this via the buffer approach as not to trigger immediate re-assembly
        let mut reconstructor = MessageReconstructor::default();
        let mut set_buf = ReconstructionBuffer::new(3);
        let mut rng = thread_rng();

        let mut message =
            vec![0u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) * 3];
        rng.fill_bytes(&mut message);

        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();
        set_buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[0]).unwrap());
        set_buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[1]).unwrap());
        set_buf.insert_fragment(Fragment::try_from_bytes(&raw_fragments[2]).unwrap());

        let set_id = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();

        reconstructor.reconstructed_sets.insert(set_id, set_buf);
        let mut reconstructor_clone = reconstructor.clone();
        let reconstructed_message = reconstructor_clone.reconstruct_message(set_id);
        assert_eq!(
            reconstructor.extract_set_payload(set_id),
            reconstructed_message.0
        );
        assert_eq!(reconstructed_message.1.len(), 1);
        assert_eq!(reconstructed_message.1[0], set_id);
    }

    #[test]
    fn reconstructing_message_for_two_sets_is_equivalent_to_combining_results_of_extracting_set_payload(
    ) {
        //
        // we're inserting this via the buffer approach as not to trigger immediate re-assembly
        let mut reconstructor = MessageReconstructor::default();
        let mut set_buf1 = ReconstructionBuffer::new(u8::MAX);
        let mut set_buf2 = ReconstructionBuffer::new(1);

        let mut rng = thread_rng();
        let mut message =
            vec![42u8; max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE) + 123];
        rng.fill_bytes(&mut message);

        let raw_fragments: Vec<_> =
            crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                .into_iter()
                .flat_map(|fragment_set| fragment_set.into_iter())
                .map(|x| x.into_bytes())
                .collect();

        for raw_fragment in raw_fragments.iter().take(u8::MAX as usize) {
            set_buf1.insert_fragment(Fragment::try_from_bytes(raw_fragment).unwrap());
        }

        set_buf2.insert_fragment(Fragment::try_from_bytes(&raw_fragments[255]).unwrap());

        let set_id1 = Fragment::try_from_bytes(&raw_fragments[0]).unwrap().id();
        let set_id2 = Fragment::try_from_bytes(&raw_fragments[255]).unwrap().id();

        reconstructor.reconstructed_sets.insert(set_id1, set_buf1);
        reconstructor.reconstructed_sets.insert(set_id2, set_buf2);
        let mut reconstructor_clone = reconstructor.clone();
        let mut reconstructor_clone2 = reconstructor.clone();

        let extracted_set1 = reconstructor.extract_set_payload(set_id1);
        let extracted_set2 = reconstructor.extract_set_payload(set_id2);

        let manually_combined_message = [extracted_set1, extracted_set2].concat();

        let reconstructed_message1 = reconstructor_clone.reconstruct_message(set_id1);
        let reconstructed_message2 = reconstructor_clone2.reconstruct_message(set_id2);

        assert_eq!(reconstructed_message1.1.len(), 2);
        assert_eq!(reconstructed_message1.1, vec![set_id1, set_id2]);

        assert_eq!(reconstructed_message2.1.len(), 2);
        assert_eq!(reconstructed_message2.1, vec![set_id1, set_id2]);

        // make sure we can use any id that is part of the message
        assert_eq!(reconstructed_message1.0, manually_combined_message);
        assert_eq!(reconstructed_message2.0, manually_combined_message);
    }

    #[test]
    fn adding_invalid_fragment_does_not_change_reconstructor_state() {
        let empty_reconstructor = MessageReconstructor::default();
        assert!(empty_reconstructor
            .recover_fragment([24u8; 43].to_vec())
            .is_err());
        assert_eq!(empty_reconstructor, MessageReconstructor::default());

        let mut reconstructor_with_data = MessageReconstructor::default();
        let dummy_message =
            vec![24u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) + 30];
        let mut fragments: Vec<_> = crate::split_into_sets(
            &mut rand::rngs::OsRng,
            &dummy_message,
            AVAILABLE_PLAINTEXT_SIZE,
        )
        .into_iter()
        .flat_map(|fragment_set| fragment_set.into_iter())
        .map(|x| x.into_bytes())
        .collect();
        reconstructor_with_data.insert_new_fragment(
            reconstructor_with_data
                .recover_fragment(fragments.pop().unwrap())
                .unwrap(),
        );
        let reconstructor_clone = reconstructor_with_data.clone();

        assert!(empty_reconstructor
            .recover_fragment([24u8; 43].to_vec())
            .is_err());
        assert_eq!(reconstructor_with_data, reconstructor_clone);
    }
}

#[cfg(test)]
mod message_reconstruction {
    use super::*;
    use rand::seq::SliceRandom;
    use rand::{thread_rng, RngCore};

    // just some arbitrary value to use in tests
    const AVAILABLE_PLAINTEXT_SIZE: usize = 1024;

    #[cfg(test)]
    mod single_set_split {
        use super::*;
        use crate::fragment::unlinked_fragment_payload_max_len;
        use crate::set::max_unlinked_set_payload_length;

        #[test]
        fn it_reconstructs_unfragmented_message() {
            let mut rng = thread_rng();

            let mut message =
                vec![0u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) - 20];
            rng.fill_bytes(&mut message);

            let fragment: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .map(|x| x.into_bytes())
                    .collect();
            assert_eq!(fragment.len(), 1);

            let mut message_reconstructor = MessageReconstructor::default();
            let reconstructed_message = message_reconstructor
                .insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragment[0].clone())
                        .unwrap(),
                )
                .unwrap();

            assert_eq!(reconstructed_message.0, message);
            assert_eq!(reconstructed_message.1.len(), 1);
        }

        #[test]
        fn it_reconstructs_unfragmented_message_of_max_length() {
            let mut rng = thread_rng();

            let mut message =
                vec![0u8; unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)];
            rng.fill_bytes(&mut message);

            let fragment: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .map(|x| x.into_bytes())
                    .collect();
            assert_eq!(fragment.len(), 1);

            let mut message_reconstructor = MessageReconstructor::default();
            let reconstructed_message = message_reconstructor
                .insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragment[0].clone())
                        .unwrap(),
                )
                .unwrap();

            assert_eq!(reconstructed_message.0, message);
            assert_eq!(reconstructed_message.1.len(), 1);
        }

        #[test]
        fn it_reconstructs_fragmented_message_in_order_of_2_max_lenghts() {
            let mut rng = thread_rng();

            let mut message =
                vec![0u8; 2 * unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)];
            rng.fill_bytes(&mut message);

            let fragments: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .map(|x| x.into_bytes())
                    .collect();
            assert_eq!(fragments.len(), 2);

            let mut message_reconstructor = MessageReconstructor::default();
            assert!(message_reconstructor
                .insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragments[0].clone())
                        .unwrap()
                )
                .is_none());

            let reconstructed_message = message_reconstructor
                .insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragments[1].clone())
                        .unwrap(),
                )
                .unwrap();

            assert_eq!(reconstructed_message.0, message);
            assert_eq!(reconstructed_message.1.len(), 1);
        }

        #[test]
        fn it_reconstructs_fragmented_message_in_order_of_with_non_max_tail() {
            let mut rng = thread_rng();

            let mut message =
                vec![0u8; 2 * unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE) - 42];
            rng.fill_bytes(&mut message);

            let fragments: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .map(|x| x.into_bytes())
                    .collect();
            assert_eq!(fragments.len(), 2);

            let mut message_reconstructor = MessageReconstructor::default();
            assert!(message_reconstructor
                .insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragments[0].clone())
                        .unwrap()
                )
                .is_none());

            let reconstructed_message = message_reconstructor
                .insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragments[1].clone())
                        .unwrap(),
                )
                .unwrap();

            assert_eq!(reconstructed_message.0, message);
            assert_eq!(reconstructed_message.1.len(), 1);
        }

        #[test]
        fn it_reconstructs_fragmented_message_in_order_of_30_fragments() {
            let mut rng = thread_rng();

            let mut message =
                vec![0u8; 30 * unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)];
            rng.fill_bytes(&mut message);

            let fragments: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .map(|x| x.into_bytes())
                    .collect();
            assert_eq!(fragments.len(), 30);

            let mut message_reconstructor = MessageReconstructor::default();

            for fragment in fragments.iter().take(fragments.len() - 1) {
                assert!(message_reconstructor
                    .insert_new_fragment(
                        message_reconstructor
                            .recover_fragment(fragment.clone())
                            .unwrap()
                    )
                    .is_none());
            }

            let reconstructed_message = message_reconstructor
                .insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragments[29].clone())
                        .unwrap(),
                )
                .unwrap();

            assert_eq!(reconstructed_message.0, message);
            assert_eq!(reconstructed_message.1.len(), 1);
        }

        #[test]
        fn it_reconstructs_fragmented_message_not_in_order_of_30_fragments() {
            let mut rng = thread_rng();

            let mut message =
                vec![0u8; 30 * unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)];
            rng.fill_bytes(&mut message);

            let mut fragments: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .map(|x| x.into_bytes())
                    .collect();
            assert_eq!(fragments.len(), 30);

            // shuffle the fragments
            fragments.shuffle(&mut rng);

            let mut message_reconstructor = MessageReconstructor::default();
            for fragment in fragments.iter().take(fragments.len() - 1) {
                assert!(message_reconstructor
                    .insert_new_fragment(
                        message_reconstructor
                            .recover_fragment(fragment.clone())
                            .unwrap()
                    )
                    .is_none());
            }

            let reconstructed_message = message_reconstructor
                .insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragments[29].clone())
                        .unwrap(),
                )
                .unwrap();

            assert_eq!(reconstructed_message.0, message);
            assert_eq!(reconstructed_message.1.len(), 1);
        }

        #[test]
        fn it_reconstructs_two_different_fragmented_messages_not_in_order_of_30_fragments_each() {
            let mut rng = thread_rng();

            let mut message1 =
                vec![0u8; 30 * unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)];
            rng.fill_bytes(&mut message1);
            let mut message2 =
                vec![0u8; 30 * unlinked_fragment_payload_max_len(AVAILABLE_PLAINTEXT_SIZE)];
            rng.fill_bytes(&mut message2);
            // introduce dummy way to identify the messages
            message1[0] = 1;
            message2[0] = 2;

            let mut fragments1: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message1, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .collect();
            assert_eq!(fragments1.len(), 30);
            let mut fragments2: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message2, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .collect();
            assert_eq!(fragments2.len(), 30);

            // combine and shuffle fragments
            fragments1.append(fragments2.as_mut());
            fragments1.shuffle(&mut rng);
            let fragments = fragments1;
            assert_eq!(fragments.len(), 60);

            let mut message_reconstructor = MessageReconstructor::default();
            for fragment in fragments {
                if let Some(reconstructed_msg) = message_reconstructor.insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragment.into_bytes())
                        .unwrap(),
                ) {
                    assert_eq!(reconstructed_msg.1.len(), 1);
                    match reconstructed_msg.0[0] {
                        1 => assert_eq!(reconstructed_msg.0, message1),
                        2 => assert_eq!(reconstructed_msg.0, message2),
                        _ => panic!("Unknown message!"),
                    }
                }
            }
        }

        #[test]
        fn it_reconstructs_two_different_messages_not_in_order_of_maximum_single_set_size_each() {
            let mut rng = thread_rng();

            let mut message1 = vec![0u8; max_unlinked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)];
            rng.fill_bytes(&mut message1);
            let mut message2 = vec![0u8; max_unlinked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)];
            rng.fill_bytes(&mut message2);
            // introduce dummy way to identify the messages
            message1[0] = 1;
            message2[0] = 2;

            let mut fragments1: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message1, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .collect();
            assert_eq!(fragments1.len(), u8::MAX as usize);
            let mut fragments2: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message2, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .collect();
            assert_eq!(fragments2.len(), u8::MAX as usize);

            // combine and shuffle fragments
            fragments1.append(fragments2.as_mut());
            fragments1.shuffle(&mut rng);
            let fragments = fragments1;
            assert_eq!(fragments.len(), (u8::MAX as usize) * 2);

            let mut message_reconstructor = MessageReconstructor::default();
            for fragment in fragments.into_iter() {
                if let Some(reconstructed_msg) = message_reconstructor.insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragment.into_bytes())
                        .unwrap(),
                ) {
                    assert_eq!(reconstructed_msg.1.len(), 1);
                    match reconstructed_msg.0[0] {
                        1 => assert_eq!(reconstructed_msg.0, message1),
                        2 => assert_eq!(reconstructed_msg.0, message2),
                        _ => panic!("Unknown message!"),
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod multiple_sets_split {
        use super::*;
        use crate::set::{
            max_one_way_linked_set_payload_length, two_way_linked_set_payload_length,
        };

        #[test]
        fn it_reconstructs_fragmented_message_not_in_order_split_into_two_sets() {
            let mut rng = thread_rng();

            let mut message =
                vec![0u8; max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE) + 12345];
            rng.fill_bytes(&mut message);

            let mut fragments: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .map(|x| x.into_bytes())
                    .collect();
            // shuffle the fragments
            fragments.shuffle(&mut rng);

            let mut message_reconstructor = MessageReconstructor::default();
            let mut finished_reconstruction = false;
            for fragment in fragments.into_iter() {
                if finished_reconstruction {
                    panic!(
                        "Shouldn't have gone into another iteration if message was reconstructed!"
                    )
                }
                if let Some(msg) = message_reconstructor
                    .insert_new_fragment(message_reconstructor.recover_fragment(fragment).unwrap())
                {
                    assert_eq!(msg.0, message);
                    assert_eq!(msg.1.len(), 2);
                    finished_reconstruction = true;
                }
            }
        }

        #[test]
        fn it_reconstructs_fragmented_message_not_in_order_split_into_four_sets() {
            let mut rng = thread_rng();

            let mut message =
                vec![
                    0u8;
                    2 * two_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                        + max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                        + 12345
                ];
            rng.fill_bytes(&mut message);

            let mut fragments: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .map(|x| x.into_bytes())
                    .collect();
            // shuffle the fragments
            fragments.shuffle(&mut rng);

            let mut message_reconstructor = MessageReconstructor::default();
            let mut finished_reconstruction = false;
            for fragment in fragments.into_iter() {
                if finished_reconstruction {
                    panic!(
                        "Shouldn't have gone into another iteration if message was reconstructed!"
                    )
                }
                if let Some(msg) = message_reconstructor
                    .insert_new_fragment(message_reconstructor.recover_fragment(fragment).unwrap())
                {
                    assert_eq!(msg.0, message);
                    assert_eq!(msg.1.len(), 4);
                    finished_reconstruction = true;
                }
            }
        }

        #[test]
        fn it_reconstructs_fragmented_message_not_in_order_split_into_four_full_sets() {
            let mut rng = thread_rng();

            let mut message =
                vec![
                    0u8;
                    2 * two_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                        + 2 * max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                ];
            rng.fill_bytes(&mut message);

            let mut fragments: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .map(|x| x.into_bytes())
                    .collect();
            assert_eq!(fragments.len(), 4 * (u8::MAX as usize));
            // shuffle the fragments
            fragments.shuffle(&mut rng);

            let mut message_reconstructor = MessageReconstructor::default();
            let mut finished_reconstruction = false;
            for fragment in fragments.into_iter() {
                if finished_reconstruction {
                    panic!(
                        "Shouldn't have gone into another iteration if message was reconstructed!"
                    )
                }
                if let Some(msg) = message_reconstructor
                    .insert_new_fragment(message_reconstructor.recover_fragment(fragment).unwrap())
                {
                    assert_eq!(msg.0, message);
                    assert_eq!(msg.1.len(), 4);
                    finished_reconstruction = true;
                }
            }
        }

        #[test]
        fn it_reconstructs_two_fragmented_messages_not_in_order_split_into_four_sets() {
            let mut rng = thread_rng();

            let mut message1 =
                vec![
                    0u8;
                    2 * two_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                        + 2 * max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                ];
            rng.fill_bytes(&mut message1);
            let mut message2 =
                vec![
                    0u8;
                    2 * two_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                        + 2 * max_one_way_linked_set_payload_length(AVAILABLE_PLAINTEXT_SIZE)
                ];
            rng.fill_bytes(&mut message2);
            // introduce dummy way to identify the messages
            message1[0] = 1;
            message2[0] = 2;

            let mut fragments1: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message1, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .collect();
            assert_eq!(fragments1.len(), 4 * (u8::MAX as usize));
            let mut fragments2: Vec<_> =
                crate::split_into_sets(&mut rand::rngs::OsRng, &message2, AVAILABLE_PLAINTEXT_SIZE)
                    .into_iter()
                    .flat_map(|fragment_set| fragment_set.into_iter())
                    .collect();
            assert_eq!(fragments2.len(), 4 * (u8::MAX as usize));

            // combine and shuffle fragments
            fragments1.append(fragments2.as_mut());
            fragments1.shuffle(&mut rng);
            let fragments = fragments1;
            assert_eq!(fragments.len(), (u8::MAX as usize) * 8);

            let mut message_reconstructor = MessageReconstructor::default();
            for fragment in fragments.into_iter() {
                if let Some(msg) = message_reconstructor.insert_new_fragment(
                    message_reconstructor
                        .recover_fragment(fragment.into_bytes())
                        .unwrap(),
                ) {
                    match msg.0[0] {
                        1 => {
                            assert_eq!(msg.0, message1);
                            assert_eq!(msg.1.len(), 4);
                        }
                        2 => {
                            assert_eq!(msg.0, message2);
                            assert_eq!(msg.1.len(), 4);
                        }
                        _ => panic!("Unknown message!"),
                    }
                }
            }
        }
    }
}
