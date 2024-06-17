// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ChunkingError;
use nym_sphinx_params::{SerializedFragmentIdentifier, FRAG_ID_LEN};

use std::fmt::{self, Debug, Formatter};

// Personal reflection: In hindsight I've spent too much time on relatively too little
// gain here, as even though I might have saved couple of bytes per packet, the gain
// is negligible in the context of having to include SURB-ACKs and reply-SURBs in the packets.
//
// However, if we really cared about those tiny optimisations, `UNLINKED_FRAGMENTED_HEADER`
// could be further compressed: if current_fragment != 1 && current_fragment != 255, you don't
// need to use the tail byte to indicate lack of linking as it can be implied from the fragment
// position.

// TODO for later: with the removal of 'unfragmented' fragments, the first bit of each header
// is completely useless, we should then think how to make the set_id become u32 instead of i32.
// (the current limitation for making the seemingly trivial change is "linked id" which
// has to have same amount of space available and right now it only has 31 bits available)

/// When the underlying message has to be split into multiple Fragments, but still manages to fit
/// into a single `FragmentSet`, each `FragmentHeader` needs to hold additional information to allow
/// for correct message reconstruction: 4 bytes for set id, 1 byte to represent total number
/// of fragments in the set, 1 byte to represent position of the current fragment in the set
/// and finally an extra byte to indicate the fragment has no links to other sets.
pub const UNLINKED_FRAGMENTED_HEADER_LEN: usize = 7;

/// Logically almost identical to `UNLINKED_FRAGMENTED_HEADER_LEN`, however, the extra three
/// bytes are due to changing the final byte that used to indicate the `Fragment` is not linked
/// into 4 byte id of either previous or the next set.
/// Note that the linked headers can potentially be used only for very first and very last
/// `Fragment` in a `FragmentSet`.
pub const LINKED_FRAGMENTED_HEADER_LEN: usize = 10;

/// Maximum size of payload of each fragment is always the maximum amount of plaintext data
/// we can put into a sphinx packet minus length of respective fragment header.
pub const fn unlinked_fragment_payload_max_len(max_plaintext_size: usize) -> usize {
    max_plaintext_size - UNLINKED_FRAGMENTED_HEADER_LEN
}

/// Maximum size of payload of each fragment is always the maximum amount of plaintext data
/// we can put into a sphinx packet minus length of respective fragment header.
pub const fn linked_fragment_payload_max_len(max_plaintext_size: usize) -> usize {
    max_plaintext_size - LINKED_FRAGMENTED_HEADER_LEN
}

// TODO: should this be defined in this module or in `cover`? I can see arguments for both options...
/// A special `FragmentIdentifier` that is not valid in all cases unless if it's used in a loop
/// cover message.
pub const COVER_FRAG_ID: FragmentIdentifier = FragmentIdentifier {
    set_id: 0,
    fragment_position: 0,
};

/// Identifier to uniquely identify a fragment. It represents 31bit ID of given `FragmentSet`
/// and u8 position of the `Fragment` in the set.
// TODO: this should really be redesigned, especially how cover and reply messages are really
// "abusing" this. They should work with it natively instead.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct FragmentIdentifier {
    set_id: i32,
    fragment_position: u8,
}

impl fmt::Display for FragmentIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Fragment Identifier: id: {} position: {}",
            self.set_id, self.fragment_position
        )
    }
}

impl FragmentIdentifier {
    pub fn to_bytes(self) -> SerializedFragmentIdentifier {
        debug_assert_eq!(FRAG_ID_LEN, 5);

        let set_id_bytes = self.set_id.to_be_bytes();
        [
            set_id_bytes[0],
            set_id_bytes[1],
            set_id_bytes[2],
            set_id_bytes[3],
            self.fragment_position,
        ]
    }

    pub fn try_from_bytes(b: SerializedFragmentIdentifier) -> Result<Self, ChunkingError> {
        debug_assert_eq!(FRAG_ID_LEN, 5);

        let set_id = i32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        // set_id == 0 is valid for COVER_FRAG_ID and replies
        if set_id < 0 {
            return Err(ChunkingError::MalformedFragmentIdentifier { received: set_id });
        }

        Ok(FragmentIdentifier {
            set_id,
            fragment_position: b[4],
        })
    }
}

/// The basic unit of division of underlying bytes message sent through the mix network.
/// Each `Fragment` after being marshaled is guaranteed to fit into a single sphinx packet.
/// The `Fragment` itself consists of part, or whole of, message to be sent as well as additional
/// header used to reconstruct the message after being received.
#[derive(PartialEq, Clone)]
pub struct Fragment {
    header: FragmentHeader,
    payload: Vec<u8>,
}

// manual implementation to hide detailed payload that we don't care about
impl Debug for Fragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fragment")
            .field("header", &self.header)
            .field("payload length", &self.payload.len())
            .finish()
    }
}

impl Fragment {
    /// Tries to encapsulate provided payload slice and metadata into a `Fragment`.
    /// It can fail if payload would not fully fit in a single `Fragment` or some of the metadata
    /// is malformed or self-contradictory, for example if current_fragment > total_fragments.
    pub(crate) fn try_new(
        payload: &[u8],
        id: i32,
        total_fragments: u8,
        current_fragment: u8,
        previous_fragments_set_id: Option<i32>,
        next_fragments_set_id: Option<i32>,
        max_plaintext_size: usize,
    ) -> Result<Self, ChunkingError> {
        let header = FragmentHeader::try_new(
            id,
            total_fragments,
            current_fragment,
            previous_fragments_set_id,
            next_fragments_set_id,
        )?;

        // check for whether payload has expected length, which depend on whether fragment is linked
        // and if it's the only one or the last one in the set (then lower bound is removed)
        let max_linked_len = linked_fragment_payload_max_len(max_plaintext_size);
        let max_unlinked_len = unlinked_fragment_payload_max_len(max_plaintext_size);

        if previous_fragments_set_id.is_some() {
            if total_fragments > 1 {
                if payload.len() != max_linked_len {
                    return Err(ChunkingError::InvalidPayloadLengthError {
                        received: payload.len(),
                        expected: max_linked_len,
                    });
                }
            } else if payload.len() > max_linked_len {
                return Err(ChunkingError::TooLongPayloadLengthError {
                    received: payload.len(),
                    expected_at_most: max_linked_len,
                });
            }
        } else if next_fragments_set_id.is_some() {
            if payload.len() != max_linked_len {
                return Err(ChunkingError::InvalidPayloadLengthError {
                    received: payload.len(),
                    expected: max_linked_len,
                });
            }
        } else if total_fragments != current_fragment {
            if payload.len() != max_unlinked_len {
                return Err(ChunkingError::InvalidPayloadLengthError {
                    received: payload.len(),
                    expected: max_unlinked_len,
                });
            }
        } else if payload.len() > max_unlinked_len {
            return Err(ChunkingError::TooLongPayloadLengthError {
                received: payload.len(),
                expected_at_most: max_unlinked_len,
            });
        }

        Ok(Fragment {
            header,
            payload: payload.to_vec(),
        })
    }

    /// based on the size of the embedded data, determines which predefined `PacketSize`
    /// was used for construction of this `Fragment`
    pub fn serialized_size(&self) -> usize {
        // TODO: optimisation: determine the size of the header without actually serializing it...
        let header_size = self.header.to_bytes().len();
        header_size + self.payload_size()
    }

    /// Convert this `Fragment` into vector of bytes which can be put into a sphinx packet.
    pub fn into_bytes(self) -> Vec<u8> {
        self.header
            .to_bytes()
            .into_iter()
            .chain(self.payload)
            .collect()
    }

    /// Derive identifier unique for this particular fragment
    pub fn fragment_identifier(&self) -> FragmentIdentifier {
        FragmentIdentifier {
            set_id: self.header.id,
            fragment_position: self.header.current_fragment,
        }
    }

    /// Gets the size of payload contained in this `Fragment`.
    pub fn payload_size(&self) -> usize {
        self.payload.len()
    }

    /// Extracts id of this `Fragment`.
    pub fn id(&self) -> i32 {
        self.header.id
    }

    /// Extracts total number of fragments associated with this particular `Fragment` (belonging to
    /// the same `FragmentSet`).
    pub fn total_fragments(&self) -> u8 {
        self.header.total_fragments
    }

    /// Extracts position of this `Fragment` in a `FragmentSet`.
    pub fn current_fragment(&self) -> u8 {
        self.header.current_fragment
    }

    /// Extracts information regarding id of pre-linked `FragmentSet`
    pub fn previous_fragments_set_id(&self) -> Option<i32> {
        self.header.previous_fragments_set_id
    }

    /// Extracts information regarding id of post-linked `FragmentSet`
    pub fn next_fragments_set_id(&self) -> Option<i32> {
        self.header.next_fragments_set_id
    }

    /// Consumes `self` to obtain payload (i.e. part of original message) associated with this
    /// `Fragment`.
    pub(crate) fn extract_payload(self) -> Vec<u8> {
        self.payload
    }

    /// Tries to recover `Fragment` from slice of bytes extracted from received sphinx packet.
    /// It can fail if payload would not fully fit in a single `Fragment` or some of the metadata
    /// is malformed or self-contradictory, for example if current_fragment > total_fragments.
    pub fn try_from_bytes(b: &[u8]) -> Result<Self, ChunkingError> {
        let (header, n) = FragmentHeader::try_from_bytes(b)?;

        // there's no sane way to decide if payload has correct range anymore as
        // it's no longer fixed

        Ok(Fragment {
            header,
            payload: b[n..].to_vec(),
        })
    }
}

/// In order to be able to re-assemble fragmented message sent through a mix-network, some
/// metadata is attached alongside the actual message payload. The idea is to include as little
/// of that data as possible due to computationally very costly process of sphinx encapsulation.
///
/// The generic `FragmentHeader` is represented as follows:
/// IF flag || 31 bit ID || TotalFragments || CurrentFragment || LID flag || 31 bit Linked ID
/// note that LID is a valid flag only for first and
/// last fragment (if TotalFragments == CurrentFragment == 255) in given set.
///
/// further note if LID is not set,
/// then the Linked ID bytes in the header are used as payload.
///
/// Hence after marshaling `FragmentHeader` into bytes,
/// the following three alternatives are possible:
///
/// 7 byte long sequence representing that this `Fragment` is one of multiple ones in the set.
/// However, the set is not linked to any other sets:
/// '1'bit || 31-bit ID || 1-byte TF || 1 byte CF || '0'byte
///
/// 10 byte sequence representing first (or last) fragment in the set,
/// where the set is linked to either preceding data (TF == 1) or proceeding data (TF == CF == 255)
/// '1'bit || 31-bit ID || 1-byte TF || 1 byte CF || '1'bit || 31-bit LID
///
/// And hence for messages larger than `max_plaintext_size` but small enough
/// to avoid set division (which happens if message has to be fragmented into more than 255 fragments)
/// there is 7 bytes of overhead inside each sphinx packet sent
/// and for the longest messages, without upper bound, there is usually also only 7 bytes
/// of overhead apart from first and last fragments in each set that instead have 10 bytes of overhead.
#[derive(PartialEq, Clone, Debug)]
pub(crate) struct FragmentHeader {
    /// ID associated with `FragmentSet` to which this particular `Fragment` belongs.
    /// Its value is restricted to (0, i32::MAX].
    /// Note that it *excludes* 0, but *includes* i32::MAX.
    /// This allows the field to be represented using 31 bits.
    id: i32,

    /// Total number of `Fragment`s in `FragmentSet` used to be able to determine if entire
    /// set was fully received as well as to perform bound checks.
    total_fragments: u8,

    /// Since message is always fragmented into payloads of constant lengths
    /// (apart from possibly the last one), there's no need to use offsets like ipv4/ipv6
    /// and we can just simply enumerate the fragments to later reconstruct the message.
    current_fragment: u8,

    /// Optional ID of previous `FragmentSet` into which the original message was split.
    /// Note, this option is only valid of `current_fragment == 1`
    previous_fragments_set_id: Option<i32>,

    /// Optional ID of next `FragmentSet` into which the original message was split.
    /// Note, this option is only valid of `current_fragment == total_fragments == u8::MAX`
    next_fragments_set_id: Option<i32>,
}

impl FragmentHeader {
    /// Tries to create a new `FragmentHeader` using provided metadata. Bunch of logical
    /// checks are performed to see if the data is not self-contradictory,
    /// for example if current_fragment > total_fragments.
    fn try_new(
        id: i32,
        total_fragments: u8,
        current_fragment: u8,
        previous_fragments_set_id: Option<i32>,
        next_fragments_set_id: Option<i32>,
    ) -> Result<Self, ChunkingError> {
        if id <= 0 {
            return Err(ChunkingError::MalformedHeaderError);
        }
        if total_fragments < current_fragment {
            return Err(ChunkingError::MalformedHeaderError);
        }
        if total_fragments == 0 {
            return Err(ChunkingError::MalformedHeaderError);
        }
        if current_fragment == 0 {
            return Err(ChunkingError::MalformedHeaderError);
        }
        if let Some(pfid) = previous_fragments_set_id {
            if pfid <= 0 || current_fragment != 1 || pfid == id {
                return Err(ChunkingError::MalformedHeaderError);
            }
        }
        if let Some(nfid) = next_fragments_set_id {
            if nfid <= 0 || current_fragment != total_fragments || nfid == id {
                return Err(ChunkingError::MalformedHeaderError);
            }
        }

        Ok(FragmentHeader {
            id,
            total_fragments,
            current_fragment,
            previous_fragments_set_id,
            next_fragments_set_id,
        })
    }

    /// Tries to recover `FragmentHeader` from slice of bytes extracted from received sphinx packet.
    /// If successful, returns `Self` and number of bytes used, as those can differ based on the
    /// type of header (unlinked or linked).
    fn try_from_bytes(b: &[u8]) -> Result<(Self, usize), ChunkingError> {
        // header needs to be at least 7 bytes long
        if b.len() < UNLINKED_FRAGMENTED_HEADER_LEN {
            return Err(ChunkingError::TooShortFragmentHeader {
                received: b.len(),
                expected: UNLINKED_FRAGMENTED_HEADER_LEN,
            });
        }
        let frag_id = i32::from_be_bytes(b[0..4].try_into().unwrap());
        // sanity check for the fragmentation flag
        if ((frag_id >> 31) & 1) == 0 {
            return Err(ChunkingError::MalformedHeaderError);
        }

        let id = frag_id & !(1 << 31); // make sure to clear the flag bit to parse id correctly
        let total_fragments = b[4];
        let current_fragment = b[5];

        if total_fragments == 0 || current_fragment == 0 || current_fragment > total_fragments {
            return Err(ChunkingError::MalformedHeaderError);
        }

        let mut previous_fragments_set_id = None;
        let mut next_fragments_set_id = None;

        // check if the linking id flag might be set
        let read_bytes = if b[6] != 0 {
            // there's linking ID supposedly attached, make sure we have enough bytes to parse
            if b.len() < LINKED_FRAGMENTED_HEADER_LEN {
                return Err(ChunkingError::TooShortFragmentHeader {
                    received: b.len(),
                    expected: LINKED_FRAGMENTED_HEADER_LEN,
                });
            }
            let flagged_linked_id = i32::from_be_bytes(b[6..10].try_into().unwrap());

            // sanity check for the linked flag
            if ((flagged_linked_id >> 31) & 1) == 0 {
                return Err(ChunkingError::MalformedHeaderError);
            }

            let linked_id = flagged_linked_id & !(1 << 31); // make sure to clear the flag bit to parse id correctly

            if current_fragment == 1 {
                previous_fragments_set_id = Some(linked_id);
            } else if total_fragments == current_fragment && current_fragment == u8::MAX {
                next_fragments_set_id = Some(linked_id);
            } else {
                return Err(ChunkingError::MalformedHeaderError);
            }

            10
        } else {
            7
        };

        Ok((
            Self::try_new(
                id,
                total_fragments,
                current_fragment,
                previous_fragments_set_id,
                next_fragments_set_id,
            )?,
            read_bytes,
        ))
    }

    /// Marshal this `FragmentHeader` into vector of bytes which can be put into a sphinx packet.
    fn to_bytes(&self) -> Vec<u8> {
        let frag_id = self.id | (1 << 31);
        let frag_id_bytes = frag_id.to_be_bytes();
        let bytes_prefix_iter = frag_id_bytes
            .into_iter()
            .chain(std::iter::once(self.total_fragments))
            .chain(std::iter::once(self.current_fragment));

        let is_linked =
            self.previous_fragments_set_id.is_some() || self.next_fragments_set_id.is_some();
        if is_linked {
            let linked_id = self
                .previous_fragments_set_id
                .unwrap_or_else(|| self.next_fragments_set_id.unwrap());
            let linked_id_entry = linked_id | (1 << 31);
            let linked_id_bytes = linked_id_entry.to_be_bytes();
            bytes_prefix_iter
                .chain(linked_id_bytes.iter().cloned())
                .collect()
        } else {
            bytes_prefix_iter.chain(std::iter::once(0)).collect()
        }
    }
}

// everything below are tests

#[cfg(test)]
mod fragment_tests {
    use super::*;
    use nym_sphinx_params::packet_sizes::PacketSize;
    use rand::{thread_rng, RngCore};

    fn max_plaintext_size() -> usize {
        PacketSize::default().plaintext_size() - PacketSize::AckPacket.size()
    }

    #[test]
    fn can_be_converted_to_and_from_bytes_for_unfragmented_payload() {
        let mut rng = thread_rng();

        let mlen = 40;
        let mut valid_message = vec![0u8; mlen];
        rng.fill_bytes(&mut valid_message);

        let valid_unfragmented_packet = Fragment {
            header: FragmentHeader::try_new(12345, 1, 1, None, None).unwrap(),
            payload: valid_message,
        };
        let packet_bytes = valid_unfragmented_packet.clone().into_bytes();
        assert_eq!(
            valid_unfragmented_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );

        let empty_unfragmented_packet = Fragment {
            header: FragmentHeader::try_new(12345, 1, 1, None, None).unwrap(),
            payload: Vec::new(),
        };
        let packet_bytes = empty_unfragmented_packet.clone().into_bytes();
        assert_eq!(
            empty_unfragmented_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );
    }

    #[test]
    fn can_be_converted_to_and_from_bytes_for_unlinked_fragmented_payload() {
        let mut rng = thread_rng();

        let mut msg = vec![0u8; unlinked_fragment_payload_max_len(max_plaintext_size())];
        rng.fill_bytes(&mut msg);

        let non_last_packet = Fragment {
            header: FragmentHeader::try_new(12345, 10, 5, None, None).unwrap(),
            payload: msg,
        };
        let packet_bytes = non_last_packet.clone().into_bytes();
        assert_eq!(
            non_last_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );

        let mut msg = vec![0u8; unlinked_fragment_payload_max_len(max_plaintext_size())];
        rng.fill_bytes(&mut msg);

        let last_full_packet = Fragment {
            header: FragmentHeader::try_new(12345, 10, 10, None, None).unwrap(),
            payload: msg,
        };
        let packet_bytes = last_full_packet.clone().into_bytes();
        assert_eq!(
            last_full_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );

        let mut msg = vec![0u8; unlinked_fragment_payload_max_len(max_plaintext_size()) - 20];
        rng.fill_bytes(&mut msg);

        let last_non_full_packet = Fragment {
            header: FragmentHeader::try_new(12345, 10, 10, None, None).unwrap(),
            payload: msg,
        };
        let packet_bytes = last_non_full_packet.clone().into_bytes();

        assert_eq!(
            last_non_full_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );
    }

    #[test]
    fn can_be_converted_to_and_from_bytes_for_pre_linked_fragmented_payload() {
        let mut rng = thread_rng();

        let mut msg = vec![0u8; linked_fragment_payload_max_len(max_plaintext_size())];
        rng.fill_bytes(&mut msg);

        let fragment = Fragment {
            header: FragmentHeader::try_new(12345, 10, 1, Some(1234), None).unwrap(),
            payload: msg,
        };
        let packet_bytes = fragment.clone().into_bytes();
        assert_eq!(fragment, Fragment::try_from_bytes(&packet_bytes).unwrap());

        let mut msg = vec![0u8; linked_fragment_payload_max_len(max_plaintext_size()) - 20];
        rng.fill_bytes(&mut msg);

        let fragment = Fragment {
            header: FragmentHeader::try_new(12345, 1, 1, Some(1234), None).unwrap(),
            payload: msg,
        };
        let packet_bytes = fragment.clone().into_bytes();
        // TODO:
        // TODO:
        // packet_bytes len assertion
        assert_eq!(fragment, Fragment::try_from_bytes(&packet_bytes).unwrap());
    }

    #[test]
    fn can_be_converted_to_and_from_bytes_for_post_linked_fragmented_payload() {
        let mut rng = thread_rng();

        let mut msg = vec![0u8; linked_fragment_payload_max_len(max_plaintext_size())];
        rng.fill_bytes(&mut msg);

        let fragment = Fragment {
            header: FragmentHeader::try_new(12345, u8::MAX, u8::MAX, None, Some(1234)).unwrap(),
            payload: msg,
        };
        let packet_bytes = fragment.clone().into_bytes();
        assert_eq!(fragment, Fragment::try_from_bytes(&packet_bytes).unwrap());

        let mut msg = vec![0u8; linked_fragment_payload_max_len(max_plaintext_size()) - 20];
        rng.fill_bytes(&mut msg);

        let fragment = Fragment {
            header: FragmentHeader::try_new(12345, u8::MAX, u8::MAX, None, Some(1234)).unwrap(),
            payload: msg,
        };
        let packet_bytes = fragment.clone().into_bytes();
        // TODO:
        // TODO:
        // packet_bytes len assertion

        assert_eq!(fragment, Fragment::try_from_bytes(&packet_bytes).unwrap());
    }

    #[test]
    fn unlinked_fragment_can_be_created_with_payload_of_valid_length() {
        let id = 12345;
        let full_payload = vec![1u8; unlinked_fragment_payload_max_len(max_plaintext_size())];
        let non_full_payload =
            vec![1u8; unlinked_fragment_payload_max_len(max_plaintext_size()) - 1];
        let non_full_payload2 =
            vec![1u8; unlinked_fragment_payload_max_len(max_plaintext_size()) - 60];

        assert!(
            Fragment::try_new(&full_payload, id, 10, 1, None, None, max_plaintext_size()).is_ok()
        );
        assert!(
            Fragment::try_new(&full_payload, id, 10, 5, None, None, max_plaintext_size()).is_ok()
        );
        assert!(
            Fragment::try_new(&full_payload, id, 10, 10, None, None, max_plaintext_size()).is_ok()
        );
        assert!(
            Fragment::try_new(&full_payload, id, 1, 1, None, None, max_plaintext_size()).is_ok()
        );

        assert!(Fragment::try_new(
            &non_full_payload,
            id,
            10,
            10,
            None,
            None,
            max_plaintext_size(),
        )
        .is_ok());
        assert!(Fragment::try_new(
            &non_full_payload,
            id,
            1,
            1,
            None,
            None,
            max_plaintext_size(),
        )
        .is_ok());

        assert!(Fragment::try_new(
            &non_full_payload2,
            id,
            10,
            10,
            None,
            None,
            max_plaintext_size(),
        )
        .is_ok());
        assert!(Fragment::try_new(
            &non_full_payload2,
            id,
            1,
            1,
            None,
            None,
            max_plaintext_size(),
        )
        .is_ok());
    }

    #[test]
    fn unlinked_fragment_returns_error_when_created_with_payload_of_invalid_length() {
        let id = 12345;
        let non_full_payload =
            vec![1u8; unlinked_fragment_payload_max_len(max_plaintext_size()) - 1];
        let non_full_payload2 =
            vec![1u8; unlinked_fragment_payload_max_len(max_plaintext_size()) - 20];
        let too_much_payload =
            vec![1u8; unlinked_fragment_payload_max_len(max_plaintext_size()) + 1];

        assert!(Fragment::try_new(
            &non_full_payload,
            id,
            10,
            1,
            None,
            None,
            max_plaintext_size(),
        )
        .is_err());
        assert!(Fragment::try_new(
            &non_full_payload,
            id,
            10,
            5,
            None,
            None,
            max_plaintext_size(),
        )
        .is_err());

        assert!(Fragment::try_new(
            &too_much_payload,
            id,
            10,
            1,
            None,
            None,
            max_plaintext_size(),
        )
        .is_err());
        assert!(Fragment::try_new(
            &too_much_payload,
            id,
            10,
            5,
            None,
            None,
            max_plaintext_size(),
        )
        .is_err());
        assert!(Fragment::try_new(
            &too_much_payload,
            id,
            1,
            1,
            None,
            None,
            max_plaintext_size(),
        )
        .is_err());

        assert!(Fragment::try_new(
            &non_full_payload2,
            id,
            10,
            1,
            None,
            None,
            max_plaintext_size(),
        )
        .is_err());
        assert!(Fragment::try_new(
            &non_full_payload2,
            id,
            10,
            5,
            None,
            None,
            max_plaintext_size(),
        )
        .is_err());
    }

    #[test]
    fn linked_fragment_can_be_created_with_payload_of_valid_length() {
        let id = 12345;
        let link_id = 1234;
        let full_payload = vec![1u8; linked_fragment_payload_max_len(max_plaintext_size())];
        let non_full_payload = vec![1u8; linked_fragment_payload_max_len(max_plaintext_size()) - 1];
        let non_full_payload2 =
            vec![1u8; linked_fragment_payload_max_len(max_plaintext_size()) - 20];

        assert!(Fragment::try_new(
            &full_payload,
            id,
            10,
            1,
            Some(link_id),
            None,
            max_plaintext_size(),
        )
        .is_ok());
        assert!(Fragment::try_new(
            &full_payload,
            id,
            1,
            1,
            Some(link_id),
            None,
            max_plaintext_size(),
        )
        .is_ok());
        assert!(Fragment::try_new(
            &non_full_payload,
            id,
            1,
            1,
            Some(link_id),
            None,
            max_plaintext_size(),
        )
        .is_ok());
        assert!(Fragment::try_new(
            &non_full_payload2,
            id,
            1,
            1,
            Some(link_id),
            None,
            max_plaintext_size(),
        )
        .is_ok());

        assert!(Fragment::try_new(
            &full_payload,
            id,
            u8::MAX,
            u8::MAX,
            None,
            Some(link_id),
            max_plaintext_size(),
        )
        .is_ok());
    }

    #[test]
    fn linked_fragment_returns_error_when_created_with_payload_of_invalid_length() {
        let id = 12345;
        let link_id = 1234;
        let non_full_payload = vec![1u8; linked_fragment_payload_max_len(max_plaintext_size()) - 1];
        let non_full_payload2 =
            vec![1u8; linked_fragment_payload_max_len(max_plaintext_size()) - 20];
        let too_much_payload = vec![1u8; linked_fragment_payload_max_len(max_plaintext_size()) + 1];

        assert!(Fragment::try_new(
            &non_full_payload,
            id,
            10,
            1,
            Some(link_id),
            None,
            max_plaintext_size(),
        )
        .is_err());
        assert!(Fragment::try_new(
            &non_full_payload2,
            id,
            10,
            1,
            Some(link_id),
            None,
            max_plaintext_size(),
        )
        .is_err());
        assert!(Fragment::try_new(
            &too_much_payload,
            id,
            10,
            1,
            Some(link_id),
            None,
            max_plaintext_size(),
        )
        .is_err());
        assert!(Fragment::try_new(
            &too_much_payload,
            id,
            1,
            1,
            Some(link_id),
            None,
            max_plaintext_size(),
        )
        .is_err());

        assert!(Fragment::try_new(
            &non_full_payload,
            id,
            u8::MAX,
            u8::MAX,
            None,
            Some(link_id),
            max_plaintext_size(),
        )
        .is_err());
        assert!(Fragment::try_new(
            &non_full_payload2,
            id,
            u8::MAX,
            u8::MAX,
            None,
            Some(link_id),
            max_plaintext_size(),
        )
        .is_err());

        assert!(Fragment::try_new(
            &too_much_payload,
            id,
            u8::MAX,
            u8::MAX,
            None,
            Some(link_id),
            max_plaintext_size(),
        )
        .is_err());
    }
}

#[cfg(test)]
mod fragment_header {
    use super::*;

    #[cfg(test)]
    mod unlinked_fragmented_payload {
        use super::*;

        #[test]
        fn can_be_converted_to_and_from_bytes_for_exact_number_of_bytes_provided() {
            let fragmented_header = FragmentHeader::try_new(12345, 10, 5, None, None).unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(UNLINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn can_be_converted_to_and_from_bytes_for_more_than_required_number_of_bytes() {
            let fragmented_header = FragmentHeader::try_new(12345, 10, 5, None, None).unwrap();

            let mut header_bytes = fragmented_header.to_bytes();
            header_bytes.append(vec![1, 2, 3, 4, 5].as_mut());

            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(UNLINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn retrieval_from_bytes_fail_for_insufficient_number_of_bytes_provided() {
            let fragmented_header = FragmentHeader::try_new(12345, 10, 5, None, None).unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let header_bytes = &header_bytes[..header_bytes.len() - 1];
            assert!(FragmentHeader::try_from_bytes(header_bytes).is_err())
        }

        #[test]
        fn retrieval_from_bytes_fail_for_invalid_fragmentation_flag() {
            let fragmented_header = FragmentHeader::try_new(10, 10, 5, None, None).unwrap();

            let mut header_bytes_low = fragmented_header.to_bytes();

            // clear the fragmentation flag
            header_bytes_low[0] &= !(1 << 7);

            let mut header_bytes_high = header_bytes_low;
            // make sure first byte of id is non-empty (apart from the fragmentation flag)
            // note for anyone reading this test in the future: choice of '3' here is arbitrary.
            header_bytes_high[0] |= 1 << 3;

            // This will have caused an error as there will be a value in the first byte
            assert!(FragmentHeader::try_from_bytes(&header_bytes_high).is_err());
        }

        #[test]
        fn retrieval_from_bytes_fail_for_invalid_link_flag() {
            let fragmented_header = FragmentHeader::try_new(12345, 10, 5, None, None).unwrap();

            let mut header_bytes = fragmented_header.to_bytes();
            // set linked flag
            header_bytes[6] |= 1 << 7;
            assert!(FragmentHeader::try_from_bytes(&header_bytes).is_err());
        }

        #[test]
        fn creation_of_header_fails_if_current_fragment_is_higher_than_total() {
            assert!(FragmentHeader::try_new(12345, 10, 11, None, None).is_err());
        }

        #[test]
        fn creation_of_header_fails_if_current_fragment_is_zero() {
            assert!(FragmentHeader::try_new(12345, 10, 0, None, None).is_err());
        }

        #[test]
        fn creation_of_header_fails_if_total_fragments_is_zero() {
            assert!(FragmentHeader::try_new(12345, 0, 0, None, None).is_err());
        }

        #[test]
        fn creation_of_header_fails_if_id_is_negative() {
            assert!(FragmentHeader::try_new(-10, 10, 5, None, None).is_err());
        }

        #[test]
        fn fragmented_header_cannot_be_created_with_zero_id() {
            assert!(FragmentHeader::try_new(0, 10, 5, None, None).is_err());
            assert!(FragmentHeader::try_new(12345, 10, 5, Some(0), None).is_err());
            assert!(FragmentHeader::try_new(12345, u8::MAX, u8::MAX, None, Some(0),).is_err());
        }

        #[test]
        fn retrieval_from_bytes_fail_if_current_fragment_is_higher_than_total() {
            // manually create header to overwrite any constructor checks
            let header = FragmentHeader {
                id: 1234,
                total_fragments: 10,
                current_fragment: 11,
                previous_fragments_set_id: None,
                next_fragments_set_id: None,
            };
            let header_bytes = header.to_bytes();
            assert!(FragmentHeader::try_from_bytes(&header_bytes).is_err());
        }

        #[test]
        fn retrieval_from_bytes_fail_if_current_or_total_fragment_is_zero() {
            // manually create header to overwrite any constructor checks
            let header = FragmentHeader {
                id: 1234,
                total_fragments: 0,
                current_fragment: 0,
                previous_fragments_set_id: None,
                next_fragments_set_id: None,
            };
            let header_bytes = header.to_bytes();
            assert!(FragmentHeader::try_from_bytes(&header_bytes).is_err());
        }
    }

    #[cfg(test)]
    mod linked_fragmented_payload {
        use super::*;

        #[test]
        fn cannot_be_linked_to_itself() {
            assert!(FragmentHeader::try_new(12345, 10, 1, Some(12345), None).is_err());
            assert!(FragmentHeader::try_new(12345, 10, 10, None, Some(12345)).is_err());
        }

        #[test]
        fn can_only_be_pre_linked_for_first_fragment() {
            assert!(FragmentHeader::try_new(12345, 10, 1, Some(1234), None).is_ok());
            assert!(FragmentHeader::try_new(12345, 10, 2, Some(1234), None).is_err());
        }

        #[test]
        fn can_only_be_post_linked_for_last_fragment() {
            assert!(FragmentHeader::try_new(12345, 10, 10, None, Some(1234)).is_ok());
            assert!(FragmentHeader::try_new(12345, u8::MAX, u8::MAX, None, Some(1234),).is_ok());
            assert!(FragmentHeader::try_new(12345, 10, 2, Some(1234), None).is_err());
        }

        #[test]
        fn pre_linked_can_be_converted_to_and_from_bytes_for_exact_number_of_bytes_provided() {
            let fragmented_header =
                FragmentHeader::try_new(12345, 10, 1, Some(1234), None).unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn pre_linked_can_be_converted_to_and_from_bytes_for_more_than_required_number_of_bytes() {
            let fragmented_header =
                FragmentHeader::try_new(12345, 10, 1, Some(1234), None).unwrap();

            let mut header_bytes = fragmented_header.to_bytes();
            header_bytes.append(vec![1, 2, 3, 4, 5].as_mut());

            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn pre_linked_is_successfully_recovered_if_its_both_first_and_final_fragment() {
            let fragmented_header = FragmentHeader::try_new(12345, 1, 1, Some(1234), None).unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn post_linked_can_be_converted_to_and_from_bytes_for_exact_number_of_bytes_provided() {
            let fragmented_header =
                FragmentHeader::try_new(12345, u8::MAX, u8::MAX, None, Some(1234)).unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn post_linked_can_be_converted_to_and_from_bytes_for_more_than_required_number_of_bytes() {
            let fragmented_header =
                FragmentHeader::try_new(12345, u8::MAX, u8::MAX, None, Some(1234)).unwrap();

            let mut header_bytes = fragmented_header.to_bytes();
            header_bytes.append(vec![1, 2, 3, 4, 5].as_mut());

            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }
    }
}
