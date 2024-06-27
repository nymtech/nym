// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::fragment::{
    linked_fragment_payload_max_len, unlinked_fragment_payload_max_len, Fragment,
    LINKED_FRAGMENTED_HEADER_LEN, UNLINKED_FRAGMENTED_HEADER_LEN,
};
use rand::Rng;

/// In the simplest case of message being divided into a single set, the set has the upper bound
/// on its payload length of the maximum number of `Fragment`s multiplied by their maximum,
/// fragmented, length.
pub const fn max_unlinked_set_payload_length(max_plaintext_size: usize) -> usize {
    u8::MAX as usize * unlinked_fragment_payload_max_len(max_plaintext_size)
}

/// If the set is being linked to another one, by either being the very first set, or the very last,
/// one of its `Fragment`s must be changed from "unlinked" into "linked" to compensate for a tiny
/// bit extra data overhead: id of the other set.
/// Note that the "MAX" prefix only applies to if the set is the last one as it does not have
/// a lower bound on its length. If the set is one way linked and a first one, it *must have*
/// this exact payload length instead.
pub const fn max_one_way_linked_set_payload_length(max_plaintext_size: usize) -> usize {
    max_unlinked_set_payload_length(max_plaintext_size)
        - (LINKED_FRAGMENTED_HEADER_LEN - UNLINKED_FRAGMENTED_HEADER_LEN)
}

/// If the set is being linked two others sets by being stuck in the middle of divided message,
/// two of its `Fragment`s (first and final one) must be changed from
/// "unlinked" into "linked" to compensate for data overhead.
/// Note that this constant no longer has a "MAX" prefix, this is because each set being stuck
/// between different sets, *must* have this exact payload length.
pub const fn two_way_linked_set_payload_length(max_plaintext_size: usize) -> usize {
    max_unlinked_set_payload_length(max_plaintext_size)
        - 2 * (LINKED_FRAGMENTED_HEADER_LEN - UNLINKED_FRAGMENTED_HEADER_LEN)
}

/// `FragmentSet` is an ordered collection of 1 to 255 `Fragment`s, each with the same ID
/// that can be used to produce original message, assuming no linking took place.
///
/// Otherwise, if set linking took place, then first or last `Fragment` from the `FragmentSet`
/// is used to determine preceding or succeeding other `FragmentSet`
/// that should be used in tandem to reconstruct original message. The linking reconstruction
/// is a recursive process as a message could have been divided into an arbitrary number of
/// `FragmentSet`s with no upper bound at all.
///
/// For example if a message was divided into 300 `Fragment`s (i.e. 2 `FragmentSet`s,
/// the structures might look as follows:
///
/// Set1: [f1 {id = 12345}, f2 {id = 12345},  ... f255 {id = 12345, next_id = 54321}]
/// Set2: [f1 {id = 54321, previous_id = 12345}, f2 {id = 54321}, ... f45 {id = 54321}]
pub(crate) type FragmentSet = Vec<Fragment>;

/// Generate a pseudo-random id for a `FragmentSet`.
/// Its value is restricted to (0, i32::MAX].
/// Note that it *excludes* 0, but *includes* i32::MAX.
/// This particular range allows for the id to be represented using 31bits, rather than
/// the full length of 32 while still providing more than enough variability to
/// distinguish different `FragmentSet`s.
/// The extra bit, as explained in `Fragment` definition is used to represents additional information,
/// indicating how further bytes should be parsed.
/// This approach saves whole byte per `Fragment`, which while may seem insignificant and
/// introduces extra complexity, quickly adds up when faced with sphinx packet encapsulation for longer
/// messages.
/// Finally, the reason 0 id is not allowed is to explicitly distinguish it from `COVER_FRAG_ID`
/// `Fragment`s thus allowing for some additional optimizations by letting it skip
/// certain procedures when reconstructing.
pub(crate) fn generate_set_id<R: Rng>(rng: &mut R) -> i32 {
    let potential_id = rng.gen::<i32>();
    // make sure id is always non-zero, as we do not want to accidentally have weird
    // reconstruction cases where unfragmented payload overwrites some part of set with id0
    // furthermore, make sure it's not i32::MIN (-2147483648) as due to 2-complement encoding,
    // attempting to calculate the absolutely value is going to panic
    if potential_id == 0 || potential_id == i32::MIN {
        generate_set_id(rng)
    } else {
        potential_id.abs()
    }
}

/// Splits underlying message into multiple `Fragment`s while all of them fit in a single
/// `Set` (number of `Fragment`s <= 255)
fn prepare_unlinked_fragmented_set(
    message: &[u8],
    id: i32,
    max_plaintext_size: usize,
) -> FragmentSet {
    let pre_casted_frags = (message.len() as f64
        / unlinked_fragment_payload_max_len(max_plaintext_size) as f64)
        .ceil() as usize;

    debug_assert!(pre_casted_frags <= u8::MAX as usize);
    let num_fragments = pre_casted_frags as u8;

    let mut fragments = Vec::with_capacity(num_fragments as usize);

    for i in 1..(pre_casted_frags + 1) {
        // we can't use u8 directly here as upper (NON-INCLUSIVE, so it would always fit) bound could be u8::MAX + 1
        let lb = (i - 1) * unlinked_fragment_payload_max_len(max_plaintext_size);
        let ub = usize::min(
            message.len(),
            i * unlinked_fragment_payload_max_len(max_plaintext_size),
        );
        fragments.push(
            Fragment::try_new(
                &message[lb..ub],
                id,
                num_fragments,
                i as u8,
                None,
                None,
                max_plaintext_size,
            )
            .unwrap(),
        )
    }

    fragments
}

/// Similarly to `prepare_unlinked_fragmented_set`, splits part of underlying message into
/// multiple `Fragment`s. The byte slice of the message *must* fit into a single linked set, however,
/// the whole message itself is still longer than a single `Set` (number of `Fragment`s > 255).
/// During the process of splitting message, this function is called multiple times.
fn prepare_linked_fragment_set(
    message: &[u8],
    id: i32,
    previous_link_id: Option<i32>,
    next_link_id: Option<i32>,
    max_plaintext_size: usize,
) -> FragmentSet {
    // determine number of fragments in the set:
    let num_frags_usize = if next_link_id.is_some() {
        u8::MAX as usize
    } else {
        // we know this set is linked, if it's not post-linked then it MUST BE pre-linked
        let tail_len = if message.len() >= linked_fragment_payload_max_len(max_plaintext_size) {
            message.len() - linked_fragment_payload_max_len(max_plaintext_size)
        } else {
            0
        };
        let pre_casted_frags = 1
            + (tail_len as f64 / unlinked_fragment_payload_max_len(max_plaintext_size) as f64)
                .ceil() as usize;
        if pre_casted_frags > u8::MAX as usize {
            panic!("message would produce too many fragments!")
        };
        pre_casted_frags
    };

    // determine bounds for the first fragment which depends on whether set is pre-linked
    let mut lb = 0;
    let mut ub = if previous_link_id.is_some() {
        usize::min(
            message.len(),
            linked_fragment_payload_max_len(max_plaintext_size),
        )
    } else {
        // the set might be linked, but fragment itself is not (i.e. the set is linked at the tail)
        unlinked_fragment_payload_max_len(max_plaintext_size)
    };

    let mut fragments = Vec::with_capacity(num_frags_usize);
    for i in 1..(num_frags_usize + 1) {
        // we can't use u8 directly here as upper (NON-INCLUSIVE, so i would always fit) bound could be u8::MAX + 1
        let fragment = Fragment::try_new(
            &message[lb..ub],
            id,
            num_frags_usize as u8,
            i as u8,
            if i == 1 { previous_link_id } else { None },
            if i == num_frags_usize {
                next_link_id
            } else {
                None
            },
            max_plaintext_size,
        )
        .unwrap();

        fragments.push(fragment);
        // update bounds for the next fragment
        lb = ub;
        ub = usize::min(
            message.len(),
            ub + unlinked_fragment_payload_max_len(max_plaintext_size),
        );
    }

    fragments
}

/// Based on total message length, determines the number of sets into which it is going to be split.
pub(crate) fn total_number_of_sets(message_len: usize, max_plaintext_size: usize) -> usize {
    if message_len <= max_unlinked_set_payload_length(max_plaintext_size) {
        1
    } else if message_len > max_unlinked_set_payload_length(max_plaintext_size)
        && message_len <= 2 * max_one_way_linked_set_payload_length(max_plaintext_size)
    {
        2
    } else {
        let len_without_edges =
            message_len - 2 * max_one_way_linked_set_payload_length(max_plaintext_size);
        // every set in between edges must be two way linked
        (len_without_edges as f64 / two_way_linked_set_payload_length(max_plaintext_size) as f64)
            .ceil() as usize
            + 2
    }
}

/// Given part of the underlying message as well id of the set as well as its potential linked sets,
/// correctly delegates to appropriate set constructor.
fn prepare_fragment_set(
    message: &[u8],
    id: i32,
    previous_link_id: Option<i32>,
    next_link_id: Option<i32>,
    max_plaintext_size: usize,
) -> FragmentSet {
    if previous_link_id.is_some() || next_link_id.is_some() {
        prepare_linked_fragment_set(
            message,
            id,
            previous_link_id,
            next_link_id,
            max_plaintext_size,
        )
    } else {
        // the bounds on whether the message fits in an unlinked set should have been done by the callee
        // when determining ids of other sets
        prepare_unlinked_fragmented_set(message, id, max_plaintext_size)
    }
}

/// Entry point for splitting whole message into possibly multiple [`Set`]s.
// TODO: make it take message: Vec<u8> instead
pub fn split_into_sets<R: Rng>(
    rng: &mut R,
    message: &[u8],
    max_plaintext_size: usize,
) -> Vec<FragmentSet> {
    let num_of_sets = total_number_of_sets(message.len(), max_plaintext_size);
    if num_of_sets == 1 {
        let set_id = generate_set_id(rng);
        vec![prepare_fragment_set(
            message,
            set_id,
            None,
            None,
            max_plaintext_size,
        )]
    } else {
        let mut sets = Vec::with_capacity(num_of_sets);
        // pre-generate all ids for the sets
        let set_ids: Vec<_> = std::iter::repeat(())
            .map(|_| generate_set_id(rng))
            .take(num_of_sets)
            .collect();

        // initial bounds for the set payloads
        let mut lb = 0;
        let mut ub = max_one_way_linked_set_payload_length(max_plaintext_size);

        for i in 0..num_of_sets {
            let fragment_set = prepare_fragment_set(
                &message[lb..ub],
                set_ids[i],
                if i == 0 { None } else { Some(set_ids[i - 1]) },
                if i == (num_of_sets - 1) {
                    None
                } else {
                    Some(set_ids[i + 1])
                },
                max_plaintext_size,
            );

            sets.push(fragment_set);
            // update bounds for the next set
            lb = ub;
            ub = if i == num_of_sets - 2 {
                // we're going to go into the last iteration now, hence the last set will be one-way linked
                usize::min(
                    message.len(),
                    ub + max_one_way_linked_set_payload_length(max_plaintext_size),
                )
            } else {
                usize::min(
                    message.len(),
                    ub + two_way_linked_set_payload_length(max_plaintext_size),
                )
            }
        }

        sets
    }
}

// reason for top level tests module is to be able to use the helper functions to verify sets payloads
#[cfg(test)]
mod tests {
    use super::*;
    use nym_sphinx_params::packet_sizes::PacketSize;

    fn max_plaintext_size() -> usize {
        PacketSize::default().plaintext_size() - PacketSize::AckPacket.size()
    }

    fn verify_unlinked_set_payload(mut set: FragmentSet, payload: &[u8]) {
        for i in (0..set.len()).rev() {
            assert_eq!(
                set.pop().unwrap().extract_payload(),
                payload[i * unlinked_fragment_payload_max_len(max_plaintext_size())
                    ..usize::min(
                        payload.len(),
                        (i + 1) * unlinked_fragment_payload_max_len(max_plaintext_size())
                    )]
                    .to_vec()
            )
        }
    }

    fn verify_pre_linked_set_payload(mut set: FragmentSet, payload: &[u8]) {
        for i in (0..set.len()).rev() {
            let lb = if i == 0 {
                0
            } else {
                (i - 1) * unlinked_fragment_payload_max_len(max_plaintext_size())
                    + linked_fragment_payload_max_len(max_plaintext_size())
            };
            let ub = usize::min(
                payload.len(),
                i * unlinked_fragment_payload_max_len(max_plaintext_size())
                    + linked_fragment_payload_max_len(max_plaintext_size()),
            );

            assert_eq!(
                set.pop().unwrap().extract_payload(),
                payload[lb..ub].to_vec()
            )
        }
    }

    fn verify_post_linked_set_payload(mut set: FragmentSet, payload: &[u8]) {
        for i in (0..set.len()).rev() {
            let lb = i * unlinked_fragment_payload_max_len(max_plaintext_size());
            let ub = if i == (u8::MAX as usize - 1) {
                i * unlinked_fragment_payload_max_len(max_plaintext_size())
                    + linked_fragment_payload_max_len(max_plaintext_size())
            } else {
                (i + 1) * unlinked_fragment_payload_max_len(max_plaintext_size())
            };

            assert_eq!(
                set.pop().unwrap().extract_payload(),
                payload[lb..ub].to_vec(),
            )
        }
    }

    fn verify_two_way_linked_set_payload(mut set: FragmentSet, payload: &[u8]) {
        for i in (0..set.len()).rev() {
            let lb = if i == 0 {
                0
            } else {
                (i - 1) * unlinked_fragment_payload_max_len(max_plaintext_size())
                    + linked_fragment_payload_max_len(max_plaintext_size())
            };
            let ub = if i == (u8::MAX as usize - 1) {
                (i - 1) * unlinked_fragment_payload_max_len(max_plaintext_size())
                    + 2 * linked_fragment_payload_max_len(max_plaintext_size())
            } else {
                i * unlinked_fragment_payload_max_len(max_plaintext_size())
                    + linked_fragment_payload_max_len(max_plaintext_size())
            };

            assert_eq!(
                set.pop().unwrap().extract_payload(),
                payload[lb..ub].to_vec(),
            )
        }
    }

    fn verify_correct_link(left: &[Fragment], right: &[Fragment]) {
        let first_id = left[0].id();
        let post_id = left[254].next_fragments_set_id().unwrap();

        let second_id = right[0].id();
        let pre_id = right[0].previous_fragments_set_id().unwrap();

        assert_eq!(first_id, pre_id);
        assert_eq!(second_id, post_id);
    }

    #[cfg(test)]
    mod preparing_unlinked_set {
        // remember this this is only called for a sole set with <= 255 fragments
        use super::*;
        use rand::{thread_rng, RngCore};

        #[test]
        fn makes_set_with_correctly_split_payload() {
            let id = 12345;
            let mut rng = thread_rng();

            let mut two_element_set_payload =
                vec![0u8; unlinked_fragment_payload_max_len(max_plaintext_size()) + 1];
            rng.fill_bytes(&mut two_element_set_payload);
            let two_element_set =
                prepare_unlinked_fragmented_set(&two_element_set_payload, id, max_plaintext_size());
            assert_eq!(2, two_element_set.len());
            verify_unlinked_set_payload(two_element_set, &two_element_set_payload);

            let mut forty_two_element_set_payload =
                vec![0u8; 41 * unlinked_fragment_payload_max_len(max_plaintext_size()) + 42];
            rng.fill_bytes(&mut forty_two_element_set_payload);
            let forty_two_element_set = prepare_unlinked_fragmented_set(
                &forty_two_element_set_payload,
                id,
                max_plaintext_size(),
            );
            assert_eq!(42, forty_two_element_set.len());
            verify_unlinked_set_payload(forty_two_element_set, &forty_two_element_set_payload);

            let mut max_fragments_set_payload =
                vec![
                    0u8;
                    max_unlinked_set_payload_length(max_plaintext_size())
                        - unlinked_fragment_payload_max_len(max_plaintext_size())
                        + 1
                ]; // last fragment should have a single byte of data
            rng.fill_bytes(&mut max_fragments_set_payload);
            let max_fragment_set = prepare_unlinked_fragmented_set(
                &max_fragments_set_payload,
                id,
                max_plaintext_size(),
            );
            assert_eq!(u8::MAX as usize, max_fragment_set.len());
            verify_unlinked_set_payload(max_fragment_set, &max_fragments_set_payload);

            let mut full_set_payload =
                vec![0u8; max_unlinked_set_payload_length(max_plaintext_size())];
            rng.fill_bytes(&mut full_set_payload);
            let full_fragment_set =
                prepare_unlinked_fragmented_set(&full_set_payload, id, max_plaintext_size());
            assert_eq!(u8::MAX as usize, full_fragment_set.len());
            verify_unlinked_set_payload(full_fragment_set, &full_set_payload);
        }

        #[test]
        #[should_panic]
        fn panics_for_too_long_payload() {
            prepare_unlinked_fragmented_set(
                &vec![0u8; max_unlinked_set_payload_length(max_plaintext_size()) + 1],
                12345,
                max_plaintext_size(),
            );
        }
    }

    #[cfg(test)]
    mod preparing_linked_set {
        use super::*;
        use rand::{thread_rng, RngCore};

        #[test]
        fn makes_set_with_correctly_split_payload_for_pre_linked_set() {
            let id = 12345;
            let link_id = 1234;
            let mut rng = thread_rng();

            let mut two_element_set_payload =
                vec![0u8; linked_fragment_payload_max_len(max_plaintext_size()) + 1];
            rng.fill_bytes(&mut two_element_set_payload);
            let two_element_set = prepare_linked_fragment_set(
                &two_element_set_payload,
                id,
                Some(link_id),
                None,
                max_plaintext_size(),
            );
            assert_eq!(2, two_element_set.len());
            verify_pre_linked_set_payload(two_element_set, &two_element_set_payload);

            let mut forty_two_element_set_payload =
                vec![
                    0u8;
                    linked_fragment_payload_max_len(max_plaintext_size())
                        + 40 * unlinked_fragment_payload_max_len(max_plaintext_size())
                        + 42
                ];
            rng.fill_bytes(&mut forty_two_element_set_payload);
            let forty_two_element_set = prepare_linked_fragment_set(
                &forty_two_element_set_payload,
                id,
                Some(link_id),
                None,
                max_plaintext_size(),
            );
            assert_eq!(42, forty_two_element_set.len());
            verify_pre_linked_set_payload(forty_two_element_set, &forty_two_element_set_payload);

            let mut max_fragments_set_payload =
                vec![
                    0u8;
                    max_unlinked_set_payload_length(max_plaintext_size())
                        - linked_fragment_payload_max_len(max_plaintext_size())
                        + 1
                ]; // last fragment should have a single byte of data
            rng.fill_bytes(&mut max_fragments_set_payload);

            let max_fragment_set = prepare_linked_fragment_set(
                &max_fragments_set_payload,
                id,
                Some(link_id),
                None,
                max_plaintext_size(),
            );
            assert_eq!(u8::MAX as usize, max_fragment_set.len());
            verify_pre_linked_set_payload(max_fragment_set, &max_fragments_set_payload);

            let mut full_set_payload =
                vec![0u8; max_one_way_linked_set_payload_length(max_plaintext_size())];
            rng.fill_bytes(&mut full_set_payload);
            let full_fragment_set = prepare_linked_fragment_set(
                &full_set_payload,
                id,
                Some(link_id),
                None,
                max_plaintext_size(),
            );
            assert_eq!(u8::MAX as usize, full_fragment_set.len());
            verify_pre_linked_set_payload(full_fragment_set, &full_set_payload);
        }

        #[test]
        #[should_panic]
        fn panics_for_too_long_payload_for_pre_linked_set() {
            prepare_linked_fragment_set(
                &vec![0u8; max_one_way_linked_set_payload_length(max_plaintext_size()) + 1],
                12345,
                Some(1234),
                None,
                max_plaintext_size(),
            );
        }

        #[test]
        fn makes_set_with_correctly_split_payload_for_post_linked_set() {
            let id = 12345;
            let link_id = 1234;
            let mut rng = thread_rng();

            // if set is post-linked, there is only a single valid case - full length payload
            let mut full_set_payload =
                vec![0u8; max_one_way_linked_set_payload_length(max_plaintext_size())];
            rng.fill_bytes(&mut full_set_payload);
            let full_fragment_set = prepare_linked_fragment_set(
                &full_set_payload,
                id,
                None,
                Some(link_id),
                max_plaintext_size(),
            );
            assert_eq!(u8::MAX as usize, full_fragment_set.len());
            verify_post_linked_set_payload(full_fragment_set, &full_set_payload);
        }

        #[test]
        #[should_panic]
        fn panics_for_too_long_payload_for_post_linked_set() {
            prepare_linked_fragment_set(
                &vec![0u8; max_one_way_linked_set_payload_length(max_plaintext_size()) + 1],
                12345,
                None,
                Some(1234),
                max_plaintext_size(),
            );
        }

        #[test]
        #[should_panic]
        fn panics_for_too_short_payload_for_post_linked_set() {
            prepare_linked_fragment_set(
                &vec![0u8; max_one_way_linked_set_payload_length(max_plaintext_size()) - 1],
                12345,
                None,
                Some(1234),
                max_plaintext_size(),
            );
        }

        #[test]
        fn makes_set_with_correctly_split_payload_for_two_way_linked_set() {
            // again, relatively simple case -
            // if set is two-way-linked, there is only a single valid case - full length payload
            let id = 12345;
            let pre_link_id = 1234;
            let post_link_id = 123456;
            let mut rng = thread_rng();

            let mut full_set_payload =
                vec![0u8; two_way_linked_set_payload_length(max_plaintext_size())];
            rng.fill_bytes(&mut full_set_payload);
            let full_fragment_set = prepare_linked_fragment_set(
                &full_set_payload,
                id,
                Some(pre_link_id),
                Some(post_link_id),
                max_plaintext_size(),
            );
            assert_eq!(u8::MAX as usize, full_fragment_set.len());
            verify_two_way_linked_set_payload(full_fragment_set, &full_set_payload);
        }

        #[test]
        #[should_panic]
        fn panics_for_too_long_payload_for_two_way_linked_set() {
            prepare_linked_fragment_set(
                &vec![0u8; two_way_linked_set_payload_length(max_plaintext_size()) + 1],
                12345,
                Some(123456),
                Some(1234),
                max_plaintext_size(),
            );
        }

        #[test]
        #[should_panic]
        fn panics_for_too_short_payload_for_two_way_linked_set() {
            prepare_linked_fragment_set(
                &vec![0u8; two_way_linked_set_payload_length(max_plaintext_size()) - 1],
                12345,
                Some(123456),
                Some(1234),
                max_plaintext_size(),
            );
        }
    }

    #[cfg(test)]
    mod splitting_into_sets {
        use super::*;
        use rand::{thread_rng, RngCore};

        #[test]
        fn correctly_creates_single_fragmented_set_when_expected() {
            let mut rng = thread_rng();
            let mut message =
                vec![0u8; max_unlinked_set_payload_length(max_plaintext_size()) - 2345];
            rng.fill_bytes(&mut message);

            let mut sets = split_into_sets(&mut rng, &message, max_plaintext_size());
            assert_eq!(1, sets.len());
            verify_unlinked_set_payload(sets.pop().unwrap(), &message);
        }

        // a very specific test case that would have saved a lot of headache if was introduced
        // earlier...
        #[test]
        fn correctly_creates_two_singly_linked_sets_with_second_set_containing_data_fitting_in_unfragmented_payload(
        ) {
            let mut rng = thread_rng();
            let mut message =
                vec![0u8; max_one_way_linked_set_payload_length(max_plaintext_size()) + 123];
            rng.fill_bytes(&mut message);
            let mut sets = split_into_sets(&mut rng, &message, max_plaintext_size());
            assert_eq!(2, sets.len());
            verify_correct_link(&sets[0], &sets[1]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[max_one_way_linked_set_payload_length(max_plaintext_size())..],
            );
            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..max_one_way_linked_set_payload_length(max_plaintext_size())],
            );
        }

        #[test]
        fn correctly_creates_two_singly_linked_sets_when_expected() {
            let mut rng = thread_rng();
            let mut message =
                vec![0u8; max_one_way_linked_set_payload_length(max_plaintext_size()) + 2345];
            rng.fill_bytes(&mut message);
            let mut sets = split_into_sets(&mut rng, &message, max_plaintext_size());
            assert_eq!(2, sets.len());
            verify_correct_link(&sets[0], &sets[1]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[max_one_way_linked_set_payload_length(max_plaintext_size())..],
            );
            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..max_one_way_linked_set_payload_length(max_plaintext_size())],
            );

            let mut message =
                vec![0u8; 2 * max_one_way_linked_set_payload_length(max_plaintext_size())];
            rng.fill_bytes(&mut message);

            let mut sets = split_into_sets(&mut rng, &message, max_plaintext_size());
            assert_eq!(2, sets.len());
            assert_eq!(sets[0].len(), u8::MAX as usize);
            assert_eq!(sets[1].len(), u8::MAX as usize);
            verify_correct_link(&sets[0], &sets[1]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[max_one_way_linked_set_payload_length(max_plaintext_size())..],
            );

            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..max_one_way_linked_set_payload_length(max_plaintext_size())],
            );
        }

        #[test]
        fn correctly_creates_four_correctly_formed_sets_when_expected() {
            let mut rng = thread_rng();
            let mut message = vec![
                0u8;
                2 * two_way_linked_set_payload_length(max_plaintext_size())
                    + max_one_way_linked_set_payload_length(max_plaintext_size())
                    + 2345
            ];
            rng.fill_bytes(&mut message);
            let mut sets = split_into_sets(&mut rng, &message, max_plaintext_size());
            assert_eq!(4, sets.len());
            assert_eq!(sets[0].len(), u8::MAX as usize);
            assert_eq!(sets[1].len(), u8::MAX as usize);
            assert_eq!(sets[2].len(), u8::MAX as usize);

            verify_correct_link(&sets[0], &sets[1]);
            verify_correct_link(&sets[1], &sets[2]);
            verify_correct_link(&sets[2], &sets[3]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[2 * two_way_linked_set_payload_length(max_plaintext_size())
                    + max_one_way_linked_set_payload_length(max_plaintext_size())..],
            );
            verify_two_way_linked_set_payload(
                sets.pop().unwrap(),
                &message[two_way_linked_set_payload_length(max_plaintext_size())
                    + max_one_way_linked_set_payload_length(max_plaintext_size())
                    ..2 * two_way_linked_set_payload_length(max_plaintext_size())
                        + max_one_way_linked_set_payload_length(max_plaintext_size())],
            );
            verify_two_way_linked_set_payload(
                sets.pop().unwrap(),
                &message[max_one_way_linked_set_payload_length(max_plaintext_size())
                    ..two_way_linked_set_payload_length(max_plaintext_size())
                        + max_one_way_linked_set_payload_length(max_plaintext_size())],
            );
            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..max_one_way_linked_set_payload_length(max_plaintext_size())],
            );

            let mut message =
                vec![
                    0u8;
                    2 * two_way_linked_set_payload_length(max_plaintext_size())
                        + 2 * max_one_way_linked_set_payload_length(max_plaintext_size())
                ];
            rng.fill_bytes(&mut message);

            let mut sets = split_into_sets(&mut rng, &message, max_plaintext_size());
            assert_eq!(4, sets.len());
            assert_eq!(sets[0].len(), u8::MAX as usize);
            assert_eq!(sets[1].len(), u8::MAX as usize);
            assert_eq!(sets[2].len(), u8::MAX as usize);
            assert_eq!(sets[3].len(), u8::MAX as usize);

            verify_correct_link(&sets[0], &sets[1]);
            verify_correct_link(&sets[1], &sets[2]);
            verify_correct_link(&sets[2], &sets[3]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[2 * two_way_linked_set_payload_length(max_plaintext_size())
                    + max_one_way_linked_set_payload_length(max_plaintext_size())..],
            );
            verify_two_way_linked_set_payload(
                sets.pop().unwrap(),
                &message[two_way_linked_set_payload_length(max_plaintext_size())
                    + max_one_way_linked_set_payload_length(max_plaintext_size())
                    ..2 * two_way_linked_set_payload_length(max_plaintext_size())
                        + max_one_way_linked_set_payload_length(max_plaintext_size())],
            );
            verify_two_way_linked_set_payload(
                sets.pop().unwrap(),
                &message[max_one_way_linked_set_payload_length(max_plaintext_size())
                    ..two_way_linked_set_payload_length(max_plaintext_size())
                        + max_one_way_linked_set_payload_length(max_plaintext_size())],
            );
            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..max_one_way_linked_set_payload_length(max_plaintext_size())],
            );
        }
    }

    #[cfg(test)]
    mod helpers {
        use super::*;

        #[test]
        fn total_number_of_sets() {
            assert_eq!(
                1,
                super::total_number_of_sets(
                    max_unlinked_set_payload_length(max_plaintext_size()) - 1,
                    max_plaintext_size()
                )
            );
            assert_eq!(
                1,
                super::total_number_of_sets(
                    max_unlinked_set_payload_length(max_plaintext_size()),
                    max_plaintext_size()
                )
            );
            assert_eq!(
                2,
                super::total_number_of_sets(
                    max_unlinked_set_payload_length(max_plaintext_size()) + 1,
                    max_plaintext_size()
                )
            );
            assert_eq!(
                2,
                super::total_number_of_sets(
                    2 * max_one_way_linked_set_payload_length(max_plaintext_size()),
                    max_plaintext_size()
                )
            );
            assert_eq!(
                3,
                super::total_number_of_sets(
                    2 * max_one_way_linked_set_payload_length(max_plaintext_size()) + 1,
                    max_plaintext_size()
                )
            );
            assert_eq!(
                3,
                super::total_number_of_sets(
                    2 * max_one_way_linked_set_payload_length(max_plaintext_size())
                        + two_way_linked_set_payload_length(max_plaintext_size())
                        - 1,
                    max_plaintext_size()
                )
            );
            assert_eq!(
                3,
                super::total_number_of_sets(
                    2 * max_one_way_linked_set_payload_length(max_plaintext_size())
                        + two_way_linked_set_payload_length(max_plaintext_size()),
                    max_plaintext_size()
                )
            );
            assert_eq!(
                4,
                super::total_number_of_sets(
                    2 * max_one_way_linked_set_payload_length(max_plaintext_size())
                        + two_way_linked_set_payload_length(max_plaintext_size())
                        + 1,
                    max_plaintext_size()
                )
            );
            assert_eq!(
                4,
                super::total_number_of_sets(
                    2 * max_one_way_linked_set_payload_length(max_plaintext_size())
                        + 2 * two_way_linked_set_payload_length(max_plaintext_size())
                        - 1,
                    max_plaintext_size()
                )
            );
            assert_eq!(
                4,
                super::total_number_of_sets(
                    2 * max_one_way_linked_set_payload_length(max_plaintext_size())
                        + 2 * two_way_linked_set_payload_length(max_plaintext_size()),
                    max_plaintext_size()
                )
            );
            assert_eq!(
                5,
                super::total_number_of_sets(
                    2 * max_one_way_linked_set_payload_length(max_plaintext_size())
                        + 2 * two_way_linked_set_payload_length(max_plaintext_size())
                        + 1,
                    max_plaintext_size()
                )
            );
        }
    }
}
