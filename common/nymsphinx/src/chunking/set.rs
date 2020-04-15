// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::chunking::fragment::{
    Fragment, LINKED_FRAGMENTED_HEADER_LEN, LINKED_FRAGMENTED_PAYLOAD_MAX_LEN,
    UNFRAGMENTED_PAYLOAD_MAX_LEN, UNLINKED_FRAGMENTED_HEADER_LEN,
    UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN,
};
use rand::Rng;

/// In the simplest case of message being divided into a single set, the set has the upper bound
/// on its payload length of the maximum number of `Fragment`s multiplied by their maximum,
/// fragmented, length.
pub const MAX_UNLINKED_SET_PAYLOAD_LENGTH: usize =
    u8::max_value() as usize * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN;

/// If the set is being linked to another one, by either being the very first set, or the very last,
/// one of its `Fragment`s must be changed from "unlinked" into "linked" to compensate for a tiny
/// bit extra data overhead: id of the other set.
/// Note that the "MAX" prefix only applies to if the set is the last one as it does not have
/// a lower bound on its length. If the set is one way linked and a first one, it *must have*
/// this exact payload length instead.
pub const MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH: usize = MAX_UNLINKED_SET_PAYLOAD_LENGTH
    - (LINKED_FRAGMENTED_HEADER_LEN - UNLINKED_FRAGMENTED_HEADER_LEN);

/// If the set is being linked two others sets by being stuck in the middle of divided message,
/// two of its `Fragment`s (first and final one) must be changed from
/// "unlinked" into "linked" to compensate for data overhead.
/// Note that this constant no longer has a "MAX" prefix, this is because each set being stuck
/// between different sets, *must* have this exact payload length.
pub const TWO_WAY_LINKED_SET_PAYLOAD_LENGTH: usize = MAX_UNLINKED_SET_PAYLOAD_LENGTH
    - 2 * (LINKED_FRAGMENTED_HEADER_LEN - UNLINKED_FRAGMENTED_HEADER_LEN);

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
/// Its value is restricted to (0, i32::max_value()].
/// Note that it *excludes* 0, but *includes* i32::max_value().
/// This particular range allows for the id to be represented using 31bits, rather than
/// the full length of 32 while still providing more than enough variability to
/// distinguish different `FragmentSet`s.
/// The extra bit, as explained in `Fragment` definition is used to represents additional information,
/// indicating how further bytes should be parsed.
/// This approach saves whole byte per `Fragment`, which while may seem insignificant and
/// introduces extra complexity, quickly adds up when faced with sphinx packet encapsulation for longer
/// messages.
/// Finally, the reason 0 id is not allowed is to explicitly distinguish it from unfragmented
/// `Fragment`s thus allowing for some additional optimizations by letting it skip
/// certain procedures when reconstructing.
fn generate_set_id<R: Rng>(rng: &mut R) -> i32 {
    let potential_id = rng.gen::<i32>().abs();
    // make sure id is always non-zero, as we do not want to accidentally have weird
    // reconstruction cases where unfragmented payload overwrites some part of set with id0
    if potential_id == 0 {
        generate_set_id(rng)
    } else {
        potential_id
    }
}

/// The simplest case of splitting underlying message - when it fits into a single
/// `Fragment` thus requiring no linking or even a set id.
/// For obvious reasons the most efficient approach.
fn prepare_unfragmented_set(message: &[u8]) -> FragmentSet {
    vec![Fragment::try_new_unfragmented(&message).unwrap()]
}

/// Splits underlying message into multiple `Fragment`s while all of them fit in a single
/// `Set` (number of `Fragment`s <= 255)
fn prepare_unlinked_fragmented_set(message: &[u8], id: i32) -> FragmentSet {
    let pre_casted_frags =
        (message.len() as f64 / UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN as f64).ceil() as usize;

    debug_assert!(pre_casted_frags <= u8::max_value() as usize);
    let num_fragments = pre_casted_frags as u8;

    let mut fragments = Vec::with_capacity(num_fragments as usize);

    for i in 1..(pre_casted_frags + 1) {
        // we can't use u8 directly here as upper (NON-INCLUSIVE, so it would always fit) bound could be u8::max_value() + 1
        let lb = (i as usize - 1) * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN;
        let ub = usize::min(
            message.len(),
            i as usize * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN,
        );
        fragments.push(
            Fragment::try_new_fragmented(&message[lb..ub], id, num_fragments, i as u8, None, None)
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
) -> FragmentSet {
    // determine number of fragments in the set:
    let num_frags_usize = if next_link_id.is_some() {
        u8::max_value() as usize
    } else {
        // we know this set is linked, if it's not post-linked then it MUST BE pre-linked
        let tail_len = if message.len() >= LINKED_FRAGMENTED_PAYLOAD_MAX_LEN {
            message.len() - LINKED_FRAGMENTED_PAYLOAD_MAX_LEN
        } else {
            0
        };
        let pre_casted_frags =
            1 + (tail_len as f64 / UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN as f64).ceil() as usize;
        if pre_casted_frags > u8::max_value() as usize {
            panic!("message would produce too many fragments!")
        };
        pre_casted_frags
    };

    // determine bounds for the first fragment which depends on whether set is pre-linked
    let mut lb = 0;
    let mut ub = if previous_link_id.is_some() {
        usize::min(message.len(), LINKED_FRAGMENTED_PAYLOAD_MAX_LEN)
    } else {
        // the set might be linked, but fragment itself is not (i.e. the set is linked at the tail)
        UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN
    };

    let mut fragments = Vec::with_capacity(num_frags_usize);
    for i in 1..(num_frags_usize + 1) {
        // we can't use u8 directly here as upper (NON-INCLUSIVE, so i would always fit) bound could be u8::max_value() + 1
        let fragment = Fragment::try_new_fragmented(
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
        )
        .unwrap();

        fragments.push(fragment);
        // update bounds for the next fragment
        lb = ub;
        ub = usize::min(message.len(), ub + UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN);
    }

    fragments
}

/// Based on total message length, determines the number of sets into which it is going to be split.
fn total_number_of_sets(message_len: usize) -> usize {
    if message_len <= MAX_UNLINKED_SET_PAYLOAD_LENGTH {
        1
    } else if message_len > MAX_UNLINKED_SET_PAYLOAD_LENGTH
        && message_len <= 2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
    {
        2
    } else {
        let len_without_edges = message_len - 2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH;
        // every set in between edges must be two way linked
        (len_without_edges as f64 / TWO_WAY_LINKED_SET_PAYLOAD_LENGTH as f64).ceil() as usize + 2
    }
}

/// Given part of the underlying message as well id of the set as well as its potential linked sets,
/// correctly delegates to appropriate set constructor.
fn prepare_fragment_set(
    message: &[u8],
    id: i32,
    previous_link_id: Option<i32>,
    next_link_id: Option<i32>,
) -> FragmentSet {
    if previous_link_id.is_some() || next_link_id.is_some() {
        prepare_linked_fragment_set(message, id, previous_link_id, next_link_id)
    } else if message.len() > UNFRAGMENTED_PAYLOAD_MAX_LEN {
        // the bounds on whether the message fits in an unlinked set should have been done by the callee
        // when determining ids of other sets
        prepare_unlinked_fragmented_set(message, id)
    } else {
        prepare_unfragmented_set(message)
    }
}

/// Entry point for splitting whole message into possibly multiple `Set`s.
pub(crate) fn split_into_sets(message: &[u8]) -> Vec<FragmentSet> {
    use rand::thread_rng;

    let mut rng = thread_rng();
    let num_of_sets = total_number_of_sets(message.len());
    if num_of_sets == 1 {
        let set_id = generate_set_id(&mut rng);
        vec![prepare_fragment_set(message, set_id, None, None)]
    } else {
        let mut sets = Vec::with_capacity(num_of_sets);
        // pre-generate all ids for the sets
        let set_ids: Vec<_> = std::iter::repeat(())
            .map(|_| generate_set_id(&mut rng))
            .take(num_of_sets)
            .collect();

        // initial bounds for the set payloads
        let mut lb = 0;
        let mut ub = MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH;

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
            );

            sets.push(fragment_set);
            // update bounds for the next set
            lb = ub;
            ub = if i == num_of_sets - 2 {
                // we're going to go into the last iteration now, hence the last set will be one-way linked
                usize::min(message.len(), ub + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH)
            } else {
                usize::min(message.len(), ub + TWO_WAY_LINKED_SET_PAYLOAD_LENGTH)
            }
        }

        sets
    }
}

// reason for top level tests module is to be able to use the helper functions to verify sets payloads
#[cfg(test)]
mod tests {
    use super::*;

    fn verify_unlinked_set_payload(mut set: FragmentSet, payload: &[u8]) {
        for i in (0..set.len()).rev() {
            assert_eq!(
                set.pop().unwrap().extract_payload(),
                payload[i * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN
                    ..usize::min(payload.len(), (i + 1) * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN)]
                    .to_vec()
            )
        }
    }

    fn verify_pre_linked_set_payload(mut set: FragmentSet, payload: &[u8]) {
        for i in (0..set.len()).rev() {
            let lb = if i == 0 {
                0
            } else {
                (i - 1) * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + LINKED_FRAGMENTED_PAYLOAD_MAX_LEN
            };
            let ub = usize::min(
                payload.len(),
                i * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + LINKED_FRAGMENTED_PAYLOAD_MAX_LEN,
            );

            assert_eq!(
                set.pop().unwrap().extract_payload(),
                payload[lb..ub].to_vec()
            )
        }
    }

    fn verify_post_linked_set_payload(mut set: FragmentSet, payload: &[u8]) {
        for i in (0..set.len()).rev() {
            let lb = i * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN;
            let ub = if i == (u8::max_value() as usize - 1) {
                i * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + LINKED_FRAGMENTED_PAYLOAD_MAX_LEN
            } else {
                (i + 1) * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN
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
                (i - 1) * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + LINKED_FRAGMENTED_PAYLOAD_MAX_LEN
            };
            let ub = if i == (u8::max_value() as usize - 1) {
                (i - 1) * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN
                    + 2 * LINKED_FRAGMENTED_PAYLOAD_MAX_LEN
            } else {
                i * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + LINKED_FRAGMENTED_PAYLOAD_MAX_LEN
            };

            assert_eq!(
                set.pop().unwrap().extract_payload(),
                payload[lb..ub].to_vec(),
            )
        }
    }

    fn verfiy_correct_link(left: &FragmentSet, right: &FragmentSet) {
        let first_id = left[0].id();
        let post_id = left[254].next_fragments_set_id().unwrap();

        let second_id = right[0].id();
        let pre_id = right[0].previous_fragments_set_id().unwrap();

        assert_eq!(first_id, pre_id);
        assert_eq!(second_id, post_id);
    }

    #[cfg(test)]
    mod preparing_unfragmented_set {
        use super::*;

        #[test]
        fn makes_set_with_single_unfragmented_element_for_valid_message_lengths() {
            let mut set = prepare_unfragmented_set(&[1]);
            assert_eq!(1, set.len());
            assert_eq!(set.pop().unwrap().extract_payload(), [1].to_vec());

            let mut set = prepare_unfragmented_set(&[1u8; UNFRAGMENTED_PAYLOAD_MAX_LEN]);
            assert_eq!(1, set.len());
            assert_eq!(
                set.pop().unwrap().extract_payload(),
                [1u8; UNFRAGMENTED_PAYLOAD_MAX_LEN].to_vec()
            );
        }

        #[test]
        #[should_panic]
        fn panics_for_too_long_payload() {
            prepare_unfragmented_set(&[1u8; UNFRAGMENTED_PAYLOAD_MAX_LEN + 1]);
        }
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

            let mut two_element_set_payload = [0u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 1];
            rng.fill_bytes(&mut two_element_set_payload);
            let two_element_set = prepare_unlinked_fragmented_set(&two_element_set_payload, id);
            assert_eq!(2, two_element_set.len());
            verify_unlinked_set_payload(two_element_set, &two_element_set_payload);

            let mut forty_two_element_set_payload =
                [0u8; 41 * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 42];
            rng.fill_bytes(&mut forty_two_element_set_payload);
            let forty_two_element_set =
                prepare_unlinked_fragmented_set(&forty_two_element_set_payload, id);
            assert_eq!(42, forty_two_element_set.len());
            verify_unlinked_set_payload(forty_two_element_set, &forty_two_element_set_payload);

            let mut max_fragments_set_payload =
                [0u8; MAX_UNLINKED_SET_PAYLOAD_LENGTH - UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 1]; // last fragment should have a single byte of data
            rng.fill_bytes(&mut max_fragments_set_payload);
            let max_fragment_set = prepare_unlinked_fragmented_set(&max_fragments_set_payload, id);
            assert_eq!(u8::max_value() as usize, max_fragment_set.len());
            verify_unlinked_set_payload(max_fragment_set, &max_fragments_set_payload);

            let mut full_set_payload = [0u8; MAX_UNLINKED_SET_PAYLOAD_LENGTH];
            rng.fill_bytes(&mut full_set_payload);
            let full_fragment_set = prepare_unlinked_fragmented_set(&full_set_payload, id);
            assert_eq!(u8::max_value() as usize, full_fragment_set.len());
            verify_unlinked_set_payload(full_fragment_set, &full_set_payload);
        }

        #[test]
        #[should_panic]
        fn panics_for_too_long_payload() {
            prepare_unlinked_fragmented_set(&[0u8; MAX_UNLINKED_SET_PAYLOAD_LENGTH + 1], 12345);
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

            let mut two_element_set_payload = [0u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 1];
            rng.fill_bytes(&mut two_element_set_payload);
            let two_element_set =
                prepare_linked_fragment_set(&two_element_set_payload, id, Some(link_id), None);
            assert_eq!(2, two_element_set.len());
            verify_pre_linked_set_payload(two_element_set, &two_element_set_payload);

            let mut forty_two_element_set_payload = [0u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN
                + 40 * UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN
                + 42];
            rng.fill_bytes(&mut forty_two_element_set_payload);
            let forty_two_element_set = prepare_linked_fragment_set(
                &forty_two_element_set_payload,
                id,
                Some(link_id),
                None,
            );
            assert_eq!(42, forty_two_element_set.len());
            verify_pre_linked_set_payload(forty_two_element_set, &forty_two_element_set_payload);

            let mut max_fragments_set_payload =
                [0u8; MAX_UNLINKED_SET_PAYLOAD_LENGTH - LINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 1]; // last fragment should have a single byte of data
            rng.fill_bytes(&mut max_fragments_set_payload);

            let max_fragment_set =
                prepare_linked_fragment_set(&max_fragments_set_payload, id, Some(link_id), None);
            assert_eq!(u8::max_value() as usize, max_fragment_set.len());
            verify_pre_linked_set_payload(max_fragment_set, &max_fragments_set_payload);

            let mut full_set_payload = [0u8; MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH];
            rng.fill_bytes(&mut full_set_payload);
            let full_fragment_set =
                prepare_linked_fragment_set(&full_set_payload, id, Some(link_id), None);
            assert_eq!(u8::max_value() as usize, full_fragment_set.len());
            verify_pre_linked_set_payload(full_fragment_set, &full_set_payload);
        }

        #[test]
        #[should_panic]
        fn panics_for_too_long_payload_for_pre_linked_set() {
            prepare_linked_fragment_set(
                &[0u8; MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH + 1],
                12345,
                Some(1234),
                None,
            );
        }

        #[test]
        fn makes_set_with_correctly_split_payload_for_post_linked_set() {
            let id = 12345;
            let link_id = 1234;
            let mut rng = thread_rng();

            // if set is post-linked, there is only a single valid case - full length payload
            let mut full_set_payload = [0u8; MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH];
            rng.fill_bytes(&mut full_set_payload);
            let full_fragment_set =
                prepare_linked_fragment_set(&full_set_payload, id, None, Some(link_id));
            assert_eq!(u8::max_value() as usize, full_fragment_set.len());
            verify_post_linked_set_payload(full_fragment_set, &full_set_payload);
        }

        #[test]
        #[should_panic]
        fn panics_for_too_long_payload_for_post_linked_set() {
            prepare_linked_fragment_set(
                &[0u8; MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH + 1],
                12345,
                None,
                Some(1234),
            );
        }

        #[test]
        #[should_panic]
        fn panics_for_too_short_payload_for_post_linked_set() {
            prepare_linked_fragment_set(
                &[0u8; MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH - 1],
                12345,
                None,
                Some(1234),
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

            let mut full_set_payload = [0u8; TWO_WAY_LINKED_SET_PAYLOAD_LENGTH];
            rng.fill_bytes(&mut full_set_payload);
            let full_fragment_set = prepare_linked_fragment_set(
                &full_set_payload,
                id,
                Some(pre_link_id),
                Some(post_link_id),
            );
            assert_eq!(u8::max_value() as usize, full_fragment_set.len());
            verify_two_way_linked_set_payload(full_fragment_set, &full_set_payload);
        }

        #[test]
        #[should_panic]
        fn panics_for_too_long_payload_for_two_way_linked_set() {
            prepare_linked_fragment_set(
                &[0u8; TWO_WAY_LINKED_SET_PAYLOAD_LENGTH + 1],
                12345,
                Some(123456),
                Some(1234),
            );
        }

        #[test]
        #[should_panic]
        fn panics_for_too_short_payload_for_two_way_linked_set() {
            prepare_linked_fragment_set(
                &[0u8; TWO_WAY_LINKED_SET_PAYLOAD_LENGTH - 1],
                12345,
                Some(123456),
                Some(1234),
            );
        }
    }

    #[cfg(test)]
    mod splitting_into_sets {
        use super::*;
        use rand::{thread_rng, RngCore};

        #[test]
        fn correctly_creates_set_with_single_unfragmented_element_when_expected() {
            let mut rng = thread_rng();
            let tiny_message = [1, 2, 3, 4, 5];
            let mut max_unfragmented_message = [0u8; UNFRAGMENTED_PAYLOAD_MAX_LEN];
            rng.fill_bytes(&mut max_unfragmented_message);

            let mut sets = split_into_sets(&tiny_message);
            assert_eq!(1, sets.len());
            assert_eq!(1, sets[0].len());
            assert_eq!(
                tiny_message.to_vec(),
                sets.pop().unwrap().pop().unwrap().extract_payload()
            );

            let mut sets = split_into_sets(&max_unfragmented_message);
            assert_eq!(1, sets.len());
            assert_eq!(1, sets[0].len());
            assert_eq!(
                max_unfragmented_message.to_vec(),
                sets.pop().unwrap().pop().unwrap().extract_payload()
            );
        }

        #[test]
        fn correctly_creates_single_fragmented_set_when_expected() {
            let mut rng = thread_rng();
            let mut message = [0u8; MAX_UNLINKED_SET_PAYLOAD_LENGTH - 2345];
            rng.fill_bytes(&mut message);

            let mut sets = split_into_sets(&message);
            assert_eq!(1, sets.len());
            verify_unlinked_set_payload(sets.pop().unwrap(), &message);
        }

        // a very specific test case that would have saved a lot of headache if was introduced
        // earlier...
        #[test]
        fn correctly_creates_two_singly_linked_sets_with_second_set_containing_data_fitting_in_unfragmented_payload(
        ) {
            let mut rng = thread_rng();
            let mut message = [0u8; MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH + 123];
            rng.fill_bytes(&mut message);
            let mut sets = split_into_sets(&message);
            assert_eq!(2, sets.len());
            verfiy_correct_link(&sets[0], &sets[1]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH..],
            );
            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH],
            );
        }

        #[test]
        fn correctly_creates_two_singly_linked_sets_when_expected() {
            let mut rng = thread_rng();
            let mut message = [0u8; MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH + 2345];
            rng.fill_bytes(&mut message);
            let mut sets = split_into_sets(&message);
            assert_eq!(2, sets.len());
            verfiy_correct_link(&sets[0], &sets[1]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH..],
            );
            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH],
            );

            let mut message = [0u8; 2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH];
            rng.fill_bytes(&mut message);

            let mut sets = split_into_sets(&message);
            assert_eq!(2, sets.len());
            assert_eq!(sets[0].len(), u8::max_value() as usize);
            assert_eq!(sets[1].len(), u8::max_value() as usize);
            verfiy_correct_link(&sets[0], &sets[1]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH..],
            );

            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH],
            );
        }

        #[test]
        fn correctly_creates_four_correctly_formed_sets_when_expected() {
            let mut rng = thread_rng();
            let mut message = [0u8; 2 * TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
                + 2345];
            rng.fill_bytes(&mut message);
            let mut sets = split_into_sets(&message);
            assert_eq!(4, sets.len());
            assert_eq!(sets[0].len(), u8::max_value() as usize);
            assert_eq!(sets[1].len(), u8::max_value() as usize);
            assert_eq!(sets[2].len(), u8::max_value() as usize);

            verfiy_correct_link(&sets[0], &sets[1]);
            verfiy_correct_link(&sets[1], &sets[2]);
            verfiy_correct_link(&sets[2], &sets[3]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[2 * TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                    + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH..],
            );
            verify_two_way_linked_set_payload(
                sets.pop().unwrap(),
                &message[TWO_WAY_LINKED_SET_PAYLOAD_LENGTH + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
                    ..2 * TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                        + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH],
            );
            verify_two_way_linked_set_payload(
                sets.pop().unwrap(),
                &message[MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
                    ..TWO_WAY_LINKED_SET_PAYLOAD_LENGTH + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH],
            );
            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH],
            );

            let mut message = [0u8; 2 * TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                + 2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH];
            rng.fill_bytes(&mut message);

            let mut sets = split_into_sets(&message);
            assert_eq!(4, sets.len());
            assert_eq!(sets[0].len(), u8::max_value() as usize);
            assert_eq!(sets[1].len(), u8::max_value() as usize);
            assert_eq!(sets[2].len(), u8::max_value() as usize);
            assert_eq!(sets[3].len(), u8::max_value() as usize);

            verfiy_correct_link(&sets[0], &sets[1]);
            verfiy_correct_link(&sets[1], &sets[2]);
            verfiy_correct_link(&sets[2], &sets[3]);
            verify_pre_linked_set_payload(
                sets.pop().unwrap(),
                &message[2 * TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                    + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH..],
            );
            verify_two_way_linked_set_payload(
                sets.pop().unwrap(),
                &message[TWO_WAY_LINKED_SET_PAYLOAD_LENGTH + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
                    ..2 * TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                        + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH],
            );
            verify_two_way_linked_set_payload(
                sets.pop().unwrap(),
                &message[MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
                    ..TWO_WAY_LINKED_SET_PAYLOAD_LENGTH + MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH],
            );
            verify_post_linked_set_payload(
                sets.pop().unwrap(),
                &message[..MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH],
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
                super::total_number_of_sets(MAX_UNLINKED_SET_PAYLOAD_LENGTH - 1)
            );
            assert_eq!(
                1,
                super::total_number_of_sets(MAX_UNLINKED_SET_PAYLOAD_LENGTH)
            );
            assert_eq!(
                2,
                super::total_number_of_sets(MAX_UNLINKED_SET_PAYLOAD_LENGTH + 1)
            );
            assert_eq!(
                2,
                super::total_number_of_sets(2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH)
            );
            assert_eq!(
                3,
                super::total_number_of_sets(2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH + 1)
            );
            assert_eq!(
                3,
                super::total_number_of_sets(
                    2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH + TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                        - 1
                )
            );
            assert_eq!(
                3,
                super::total_number_of_sets(
                    2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH + TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                )
            );
            assert_eq!(
                4,
                super::total_number_of_sets(
                    2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
                        + TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                        + 1
                )
            );
            assert_eq!(
                4,
                super::total_number_of_sets(
                    2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
                        + 2 * TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                        - 1
                )
            );
            assert_eq!(
                4,
                super::total_number_of_sets(
                    2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
                        + 2 * TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                )
            );
            assert_eq!(
                5,
                super::total_number_of_sets(
                    2 * MAX_ONE_WAY_LINKED_SET_PAYLOAD_LENGTH
                        + 2 * TWO_WAY_LINKED_SET_PAYLOAD_LENGTH
                        + 1
                )
            );
        }
    }
}
