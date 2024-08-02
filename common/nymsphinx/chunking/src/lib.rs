// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::fragment::{linked_fragment_payload_max_len, unlinked_fragment_payload_max_len};
pub use set::split_into_sets;
use thiserror::Error;

pub const MIN_PADDING_OVERHEAD: usize = 1;

// Future consideration: currently in a lot of places, the payloads have randomised content
// which is not a perfect testing strategy as it might not detect some edge cases I never would
// have assumed could be possible. A better approach would be to research some Fuzz testing
// library like: https://github.com/rust-fuzz/afl.rs and use that instead for the inputs.

// perhaps it might be useful down the line for interaction testing between client,mixes,etc?

// TODO: this module has evolved significantly since the tests were first written
// they should definitely be revisited.
// For instance there are not tests for the cases when we are padding the message

pub mod fragment;
pub mod reconstruction;
pub mod set;

/// The idea behind the process of chunking is to incur as little data overhead as possible due
/// to very computationally costly sphinx encapsulation procedure.
///
/// To achieve this, the underlying message is split into so-called "sets", which are further
/// subdivided into the base unit of "fragment" that is directly encapsulated by a Sphinx packet.
/// This allows to encapsulate messages of arbitrary length.
///
/// Each message, regardless of its size, consists of at least a single `Set` that has at least
/// a single `Fragment`.
///
/// Each `Fragment` can have variable, yet fully deterministic, length,
/// that depends on its position in the set as well as total number of sets. This is further
/// explained in `fragment.rs` file.  
///
/// Similarly, each `Set` can have a variable number of `Fragment`s inside. However, that
/// value is more restrictive: if it's the last set into which the message was split
/// (or implicitly the only one), it has no lower bound on the number of `Fragment`s.
/// (Apart from the restriction of containing at least a single one). If the set is located
/// somewhere in the middle, *it must be* full. Finally, regardless of its position, it must also be
/// true that it contains no more than `u8::MAX`, i.e. 255 `Fragment`s.
/// Again, the reasoning for this is further explained in `set.rs` file. However, you might
/// also want to look at `fragment.rs` to understand the full context behind that design choice.
///
/// Both of those concepts as well as their structures, i.e. `Set` and `Fragment`
/// are further explained in the respective files.

#[derive(PartialEq, Eq, Debug, Error)]
pub enum ChunkingError {
    #[error("Received payload is too long. Got {received}, expected {expected}")]
    InvalidPayloadLengthError { received: usize, expected: usize },

    #[error("Received payload is too long. Got {received}, expected at most {expected_at_most}")]
    TooLongPayloadLengthError {
        received: usize,
        expected_at_most: usize,
    },

    // this should really be split into multiple variants to provide better error information
    #[error("Provided header was malformed or contained self-contradicting fields")]
    MalformedHeaderError,

    #[error("Received too few bytes to deserialize fragment header. Got {received}, expected {expected}")]
    TooShortFragmentHeader { received: usize, expected: usize },

    #[error("Received fragment identifier ({received}) is not a valid value!")]
    MalformedFragmentIdentifier { received: i32 },
}

/// Returns number of fragments the message will be split to as well as number of available
/// bytes in the final fragment
pub fn number_of_required_fragments(
    message_len: usize,
    plaintext_per_fragment: usize,
) -> (usize, usize) {
    let max_unlinked = unlinked_fragment_payload_max_len(plaintext_per_fragment);
    let max_linked = linked_fragment_payload_max_len(plaintext_per_fragment);

    match set::total_number_of_sets(message_len, plaintext_per_fragment) {
        1 => {
            // is if it's a single fragment message
            if message_len < max_unlinked {
                return (1, max_unlinked - message_len);
            }

            // all fragments will be 'unlinked'
            let quot = message_len / max_unlinked;
            let rem = message_len % max_unlinked;

            if rem == 0 {
                (quot, 0)
            } else {
                (quot + 1, max_unlinked - rem)
            }
        }

        n => {
            // in first and last set there will be one 'linked' fragment
            // and two 'linked' fragment in every other set, meaning
            // there will be 2 * (n - 2) + 2 = 2n - 2 'linked' fragments total
            // rest will be 'unlinked'

            // we know for sure that all fragments in all but last set are definitely full
            // (last one has single 'linked' fragment)
            let without_last = (n - 1) * (u8::MAX as usize);
            let linked_fragments_without_last = (2 * n - 2) - 1;
            let unlinked_fragments_without_last = without_last - linked_fragments_without_last;

            let final_set_message_len = message_len
                - linked_fragments_without_last * max_linked
                - unlinked_fragments_without_last * max_unlinked;

            // we must be careful with the last set as it might be the case that it only
            // consists of a single, linked, non-full fragment
            match final_set_message_len {
                n if n < max_linked => (without_last + 1, max_linked - final_set_message_len),
                n if n == max_linked => (without_last + 1, 0),
                _ => {
                    let remaining_len = final_set_message_len - max_linked;

                    let quot = remaining_len / max_unlinked;
                    let rem = remaining_len % max_unlinked;

                    if rem == 0 {
                        (without_last + quot + 1, 0)
                    } else {
                        (without_last + quot + 2, max_unlinked - rem)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set::{max_one_way_linked_set_payload_length, two_way_linked_set_payload_length};
    use nym_sphinx_addressing::nodes::MAX_NODE_ADDRESS_UNPADDED_LEN;
    use nym_sphinx_params::packet_sizes::PacketSize;

    #[test]
    fn calculating_number_of_required_fragments() {
        // plaintext len should not affect this at all, but let's test it with something tiny
        // and reasonable
        let used_plaintext_len = PacketSize::default().plaintext_size()
            - PacketSize::AckPacket.size()
            - MAX_NODE_ADDRESS_UNPADDED_LEN;

        let plaintext_lens = vec![17, used_plaintext_len, 20, 42, 10000];
        const SET_LEN: usize = u8::MAX as usize;

        for plaintext_len in plaintext_lens {
            let unlinked_len = unlinked_fragment_payload_max_len(plaintext_len);
            let linked_len = linked_fragment_payload_max_len(plaintext_len);
            let full_edge_set = max_one_way_linked_set_payload_length(plaintext_len);
            let full_middle_set = two_way_linked_set_payload_length(plaintext_len);

            let single_non_full_frag_message_len = unlinked_len - 5;
            let (frags, space_left) =
                number_of_required_fragments(single_non_full_frag_message_len, plaintext_len);
            assert_eq!(frags, 1);
            assert_eq!(space_left, unlinked_len - single_non_full_frag_message_len);

            let single_full_frag_message_len = unlinked_len;
            let (frags, space_left) =
                number_of_required_fragments(single_full_frag_message_len, plaintext_len);
            assert_eq!(frags, 1);
            assert_eq!(space_left, 0);

            let two_non_full_frags_len = unlinked_len + 1;
            let (frags, space_left) =
                number_of_required_fragments(two_non_full_frags_len, plaintext_len);
            assert_eq!(frags, 2);
            assert_eq!(space_left, unlinked_len - 1);

            let two_full_frags_len = 2 * unlinked_len;
            let (frags, space_left) =
                number_of_required_fragments(two_full_frags_len, plaintext_len);
            assert_eq!(frags, 2);
            assert_eq!(space_left, 0);

            let multi_single_set_frags_non_full = unlinked_len * 42 - 5;
            let (frags, space_left) =
                number_of_required_fragments(multi_single_set_frags_non_full, plaintext_len);
            assert_eq!(frags, 42);
            assert_eq!(space_left, 5);

            let multi_single_set_frags_full = unlinked_len * 42;
            let (frags, space_left) =
                number_of_required_fragments(multi_single_set_frags_full, plaintext_len);
            assert_eq!(frags, 42);
            assert_eq!(space_left, 0);

            let two_set_one_non_full_frag = full_edge_set + linked_len - 1;
            let (frags, space_left) =
                number_of_required_fragments(two_set_one_non_full_frag, plaintext_len);
            assert_eq!(frags, SET_LEN + 1);
            assert_eq!(space_left, 1);

            let two_set_one_full_frag = full_edge_set + linked_len;
            let (frags, space_left) =
                number_of_required_fragments(two_set_one_full_frag, plaintext_len);
            assert_eq!(frags, SET_LEN + 1);
            assert_eq!(space_left, 0);

            let two_set_multi_frags_non_full = full_edge_set + linked_len + unlinked_len * 41 - 5;
            let (frags, space_left) =
                number_of_required_fragments(two_set_multi_frags_non_full, plaintext_len);
            assert_eq!(frags, SET_LEN + 42);
            assert_eq!(space_left, 5);

            let two_set_multi_frags_full = full_edge_set + linked_len + unlinked_len * 41;
            let (frags, space_left) =
                number_of_required_fragments(two_set_multi_frags_full, plaintext_len);
            assert_eq!(frags, SET_LEN + 42);
            assert_eq!(space_left, 0);

            let ten_set_one_non_full_frag = full_edge_set + 8 * full_middle_set + linked_len - 1;
            let (frags, space_left) =
                number_of_required_fragments(ten_set_one_non_full_frag, plaintext_len);
            assert_eq!(frags, 9 * SET_LEN + 1);
            assert_eq!(space_left, 1);

            let ten_set_one_full_frag = full_edge_set + 8 * full_middle_set + linked_len;
            let (frags, space_left) =
                number_of_required_fragments(ten_set_one_full_frag, plaintext_len);
            assert_eq!(frags, 9 * SET_LEN + 1);
            assert_eq!(space_left, 0);

            let ten_set_multi_frags_non_full =
                full_edge_set + 8 * full_middle_set + linked_len + 41 * unlinked_len - 5;
            let (frags, space_left) =
                number_of_required_fragments(ten_set_multi_frags_non_full, plaintext_len);
            assert_eq!(frags, 9 * SET_LEN + 42);
            assert_eq!(space_left, 5);

            let ten_set_multi_frags_full =
                full_edge_set + 8 * full_middle_set + linked_len + 41 * unlinked_len;
            let (frags, space_left) =
                number_of_required_fragments(ten_set_multi_frags_full, plaintext_len);
            assert_eq!(frags, 9 * SET_LEN + 42);
            assert_eq!(space_left, 0);
        }
    }
}
