use crate::chunking::ChunkingError;
use std::convert::TryInto;

/// The entire marshaled `Fragment` can never be longer than the maximum length of the plaintext
/// data we can put into a Sphinx packet.
pub const MAXIMUM_FRAGMENT_LENGTH: usize = sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH;

/// The minimum data overhead required for message fitting into a single `Fragment`. The single byte
/// used to literally indicate "this message is not fragmented".
pub const UNFRAGMENTED_HEADER_LEN: usize = 1;

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
pub const UNFRAGMENTED_PAYLOAD_MAX_LEN: usize = MAXIMUM_FRAGMENT_LENGTH - UNFRAGMENTED_HEADER_LEN;

/// Maximum size of payload of each fragment is always the maximum amount of plaintext data
/// we can put into a sphinx packet minus length of respective fragment header.
pub const UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN: usize =
    MAXIMUM_FRAGMENT_LENGTH - UNLINKED_FRAGMENTED_HEADER_LEN;

/// Maximum size of payload of each fragment is always the maximum amount of plaintext data
/// we can put into a sphinx packet minus length of respective fragment header.
pub const LINKED_FRAGMENTED_PAYLOAD_MAX_LEN: usize =
    MAXIMUM_FRAGMENT_LENGTH - LINKED_FRAGMENTED_HEADER_LEN;

/// The basic unit of division of underlying bytes message sent through the mix network.
/// Each `Fragment` after being marshaled is guaranteed to fit into a single sphinx packet.
/// The `Fragment` itself consists of part, or whole of, message to be sent as well as additional
/// header used to reconstruct the message after being received.
#[derive(PartialEq, Clone, Debug)]
pub(crate) struct Fragment {
    header: FragmentHeader,
    payload: Vec<u8>,
}

impl Fragment {
    /// Tries to encapsulate provided payload slice and metadata into a `Fragment`.
    /// It can fail if payload would not fully fit in a single `Fragment` or some of the metadata
    /// is malformed or self-contradictory, for example if current_fragment > total_fragments.
    pub(crate) fn try_new_fragmented(
        payload: &[u8],
        id: i32,
        total_fragments: u8,
        current_fragment: u8,
        previous_fragments_set_id: Option<i32>,
        next_fragments_set_id: Option<i32>,
    ) -> Result<Self, ChunkingError> {
        let header = FragmentHeader::try_new_fragmented(
            id,
            total_fragments,
            current_fragment,
            previous_fragments_set_id,
            next_fragments_set_id,
        )?;

        // check for whether payload has expected length, which depend on whether fragment is linked
        // and if it's the only one or the last one in the set (then lower bound is removed)
        if previous_fragments_set_id.is_some() {
            if total_fragments > 1 {
                if payload.len() != LINKED_FRAGMENTED_PAYLOAD_MAX_LEN {
                    return Err(ChunkingError::InvalidPayloadLengthError);
                }
            } else {
                if payload.len() > LINKED_FRAGMENTED_PAYLOAD_MAX_LEN {
                    return Err(ChunkingError::InvalidPayloadLengthError);
                }
            }
        } else if next_fragments_set_id.is_some() {
            if payload.len() != LINKED_FRAGMENTED_PAYLOAD_MAX_LEN {
                return Err(ChunkingError::InvalidPayloadLengthError);
            }
        } else {
            if total_fragments != current_fragment {
                if payload.len() != UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN {
                    return Err(ChunkingError::InvalidPayloadLengthError);
                }
            } else {
                if payload.len() > UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN {
                    return Err(ChunkingError::InvalidPayloadLengthError);
                }
            }
        }

        Ok(Fragment {
            header,
            payload: payload.to_vec(),
        })
    }

    /// The most efficient way of representing underlying message incurring the least data overhead
    /// for as long as the message can fit into a single, unfragmented, `Fragment`.
    pub(crate) fn try_new_unfragmented(payload: &[u8]) -> Result<Self, ChunkingError> {
        if payload.len() > UNFRAGMENTED_PAYLOAD_MAX_LEN {
            Err(ChunkingError::InvalidPayloadLengthError)
        } else {
            Ok(Fragment {
                header: FragmentHeader::new_unfragmented(),
                payload: payload.to_vec(),
            })
        }
    }

    /// Convert this `Fragment` into vector of bytes which can be put into a sphinx packet.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.header
            .to_bytes()
            .into_iter()
            .chain(self.payload.into_iter())
            .collect()
    }

    /// Extracts id of this `Fragment`.
    pub(crate) fn id(&self) -> i32 {
        self.header.id
    }

    /// Extracts total number of fragments associated with this particular `Fragment` (belonging to
    /// the same `FragmentSet`).
    pub(crate) fn total_fragments(&self) -> u8 {
        self.header.total_fragments
    }

    /// Extracts position of this `Fragment` in a `FragmentSet`.
    pub(crate) fn current_fragment(&self) -> u8 {
        self.header.current_fragment
    }

    /// Extracts information regarding id of pre-linked `FragmentSet`
    pub(crate) fn previous_fragments_set_id(&self) -> Option<i32> {
        self.header.previous_fragments_set_id
    }

    /// Extracts information regarding id of post-linked `FragmentSet`
    pub(crate) fn next_fragments_set_id(&self) -> Option<i32> {
        self.header.next_fragments_set_id
    }

    /// Consumes `Self` to obtain payload (i.e. part of original message) associated with this
    /// `Fragment`.
    pub(crate) fn extract_payload(self) -> Vec<u8> {
        self.payload
    }

    /// Tries to recover `Fragment` from slice of bytes extracted from received sphinx packet.
    /// It can fail if payload would not fully fit in a single `Fragment` or some of the metadata
    /// is malformed or self-contradictory, for example if current_fragment > total_fragments.
    pub(crate) fn try_from_bytes(b: &[u8]) -> Result<Self, ChunkingError> {
        let (header, n) = FragmentHeader::try_from_bytes(b)?;

        // determine what's our expected payload size bound and whether the message fits in this
        let is_payload_in_range = if header.is_fragmented {
            if header.is_linked() {
                if header.is_final() {
                    (b.len() - n) <= LINKED_FRAGMENTED_PAYLOAD_MAX_LEN
                } else {
                    (b.len() - n) == LINKED_FRAGMENTED_PAYLOAD_MAX_LEN
                }
            } else {
                if header.is_final() {
                    (b.len() - n) <= UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN
                } else {
                    (b.len() - n) == UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN
                }
            }
        } else {
            (b.len() - n) <= UNFRAGMENTED_PAYLOAD_MAX_LEN
        };

        if !is_payload_in_range {
            return Err(ChunkingError::MalformedFragmentData);
        }

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
/// further note that if IF (isFragmented) is not set,
/// then the remaining bytes of the header are used as payload and also note that if LID is not set,
/// then the Linked ID bytes in the header are used as payload.
///
/// Hence after marshaling `FragmentHeader` into bytes,
/// the following three alternatives are possible:
///
/// Single byte representing that the `Fragment` is the only one into which the message was split.
/// '0'byte
///
/// 7 byte long sequence representing that this `Fragment` is one of multiple ones in the set.
/// However, the set is not linked to any other sets:
/// '1'bit || 31-bit ID || 1-byte TF || 1 byte CF || '0'byte
///
/// 10 byte sequence representing first (or last) fragment in the set,
/// where the set is linked to either preceding data (TF == 1) or proceeding data (TF == CF == 255)
/// '1'bit || 31-bit ID || 1-byte TF || 1 byte CF || '1'bit || 31-bit LID
///
/// And hence for messages smaller than sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH,
/// there is only a single byte of overhead;
/// for messages larger than sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH but small enough
/// to avoid set division (which happens if message has to be fragmented into more than 255 fragments)
/// there is 7 bytes of overhead inside each sphinx packet sent
/// and finally for the longest messages, without upper bound, there is usually also only 7 bytes
/// of overhead apart from first and last fragments in each set that instead have 10 bytes of overhead.
#[derive(PartialEq, Clone, Debug)]
pub(crate) struct FragmentHeader {
    /// Flag used to indicate whether this `Fragment` is the only one into which original
    /// message was split.
    /// If so, rest of the fields are meaningless as they are set to their default values
    /// the whole time and serve no purpose when marshaling or unmarshaling a `FragmentHeader`
    is_fragmented: bool,

    /// ID associated with `FragmentSet` to which this particular `Fragment` belongs.
    /// Its value is restricted to (0, i32::max_value()].
    /// Note that it *excludes* 0, but *includes* i32::max_value().
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
    /// Note, this option is only valid of `current_fragment == total_fragments == u8::max_value()`
    next_fragments_set_id: Option<i32>,
}

impl FragmentHeader {
    /// Tries to create a new `FragmentHeader` using provided metadata. Bunch of logical
    /// checks are performed to see if the data is not self-contradictory,
    /// for example if current_fragment > total_fragments.
    fn try_new_fragmented(
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
            is_fragmented: true,
            id,
            total_fragments,
            current_fragment,
            previous_fragments_set_id,
            next_fragments_set_id,
        })
    }

    /// When unfragmented header is created, no checks need to happen as there is no variability
    /// in the values of its fields. All of them always have the same, default, values.
    fn new_unfragmented() -> Self {
        FragmentHeader {
            is_fragmented: false,
            id: 0,
            total_fragments: 1,
            current_fragment: 1,
            previous_fragments_set_id: None,
            next_fragments_set_id: None,
        }
    }

    /// Tries to recover `FragmentHeader` from slice of bytes extracted from received sphinx packet.
    /// If successful, returns `Self` and number of bytes used, as those can differ based on the
    /// type of header (unfragmented, unlinked, linked).
    fn try_from_bytes(b: &[u8]) -> Result<(Self, usize), ChunkingError> {
        if b.is_empty() {
            return Err(ChunkingError::TooShortFragmentData);
        }
        // check if it's fragmented - if it's not - the whole first byte is set to 0
        // otherwise first bit is set to 1
        if b[0] == 0 {
            Ok((Self::new_unfragmented(), 1))
        } else {
            // if it's fragmented, it needs to be at least 7 bytes long
            if b.len() < UNLINKED_FRAGMENTED_HEADER_LEN {
                return Err(ChunkingError::TooShortFragmentData);
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
                    return Err(ChunkingError::TooShortFragmentData);
                }
                let flagged_linked_id = i32::from_be_bytes(b[6..10].try_into().unwrap());

                // sanity check for the linked flag
                if ((flagged_linked_id >> 31) & 1) == 0 {
                    return Err(ChunkingError::MalformedHeaderError);
                }

                let linked_id = flagged_linked_id & !(1 << 31); // make sure to clear the flag bit to parse id correctly

                if current_fragment == 1 {
                    previous_fragments_set_id = Some(linked_id);
                } else if total_fragments == current_fragment && current_fragment == u8::max_value()
                {
                    next_fragments_set_id = Some(linked_id);
                } else {
                    return Err(ChunkingError::MalformedHeaderError);
                }

                10
            } else {
                7
            };

            Ok((
                Self::try_new_fragmented(
                    id,
                    total_fragments,
                    current_fragment,
                    previous_fragments_set_id,
                    next_fragments_set_id,
                )?,
                read_bytes,
            ))
        }
    }

    /// Helper method to determine if this `FragmentHeader` is used to represent a linked `Fragment`.
    fn is_linked(&self) -> bool {
        self.previous_fragments_set_id.is_some() || self.next_fragments_set_id.is_some()
    }

    /// Helper method to determine if this `FragmentHeader` is used to represent a `Fragment` that
    /// is a final one in some `FragmentSet`.
    fn is_final(&self) -> bool {
        self.total_fragments == self.current_fragment
    }

    /// Marshal this `FragmentHeader` into vector of bytes which can be put into a sphinx packet.
    fn to_bytes(&self) -> Vec<u8> {
        if self.is_fragmented {
            let frag_id = self.id | (1 << 31);
            let frag_id_bytes = frag_id.to_be_bytes();
            let bytes_prefix_iter = frag_id_bytes
                .iter()
                .cloned()
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
        } else {
            [0].to_vec()
        }
    }
}

// everything below are tests

#[cfg(test)]
mod fragment {
    use rand::{thread_rng, RngCore};

    use super::*;

    #[test]
    fn can_be_converted_to_and_from_bytes_for_unfragmented_payload() {
        let mut rng = thread_rng();

        let mlen = UNFRAGMENTED_PAYLOAD_MAX_LEN - 20;
        let mut valid_message = vec![0u8; mlen];
        rng.fill_bytes(&mut valid_message);

        let valid_unfragmented_packet = Fragment {
            header: FragmentHeader::new_unfragmented(),
            payload: valid_message,
        };
        let packet_bytes = valid_unfragmented_packet.clone().into_bytes();
        assert_eq!(
            valid_unfragmented_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );

        let empty_unfragmented_packet = Fragment {
            header: FragmentHeader::new_unfragmented(),
            payload: Vec::new(),
        };
        let packet_bytes = empty_unfragmented_packet.clone().into_bytes();
        assert_eq!(
            empty_unfragmented_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );

        let mut full_message = vec![0u8; UNFRAGMENTED_PAYLOAD_MAX_LEN];
        rng.fill_bytes(&mut full_message);

        let full_unfragmented_packet = Fragment {
            header: FragmentHeader::new_unfragmented(),
            payload: full_message,
        };
        let packet_bytes = full_unfragmented_packet.clone().into_bytes();
        assert_eq!(
            full_unfragmented_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );
    }

    #[test]
    fn conversion_from_bytes_fails_for_too_long_unfragmented_payload() {
        let mut rng = thread_rng();

        let mlen = UNFRAGMENTED_PAYLOAD_MAX_LEN + 1;
        let mut message = vec![0u8; mlen];
        rng.fill_bytes(&mut message);

        let packet = Fragment {
            header: FragmentHeader::new_unfragmented(),
            payload: message,
        };

        let packet_bytes = packet.into_bytes();
        assert_eq!(
            Fragment::try_from_bytes(&packet_bytes),
            Err(ChunkingError::MalformedFragmentData)
        );
    }

    #[test]
    fn can_be_converted_to_and_from_bytes_for_unlinked_fragmented_payload() {
        let mut rng = thread_rng();

        let mut msg = vec![0u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN];
        rng.fill_bytes(&mut msg);

        let non_last_packet = Fragment {
            header: FragmentHeader::try_new_fragmented(12345, 10, 5, None, None).unwrap(),
            payload: msg,
        };
        let packet_bytes = non_last_packet.clone().into_bytes();
        assert_eq!(
            non_last_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );

        let mut msg = vec![0u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN];
        rng.fill_bytes(&mut msg);

        let last_full_packet = Fragment {
            header: FragmentHeader::try_new_fragmented(12345, 10, 10, None, None).unwrap(),
            payload: msg,
        };
        let packet_bytes = last_full_packet.clone().into_bytes();
        assert_eq!(
            last_full_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );

        let mut msg = vec![0u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 20];
        rng.fill_bytes(&mut msg);

        let last_non_full_packet = Fragment {
            header: FragmentHeader::try_new_fragmented(12345, 10, 10, None, None).unwrap(),
            payload: msg,
        };
        let packet_bytes = last_non_full_packet.clone().into_bytes();
        assert_eq!(
            last_non_full_packet,
            Fragment::try_from_bytes(&packet_bytes).unwrap()
        );
    }

    #[test]
    fn conversion_from_bytes_fails_for_too_long_unlinked_fragmented_payload() {
        let mut rng = thread_rng();

        let mlen = UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 1;
        let mut message = vec![0u8; mlen];
        rng.fill_bytes(&mut message);

        let packet = Fragment {
            header: FragmentHeader::try_new_fragmented(12345, 10, 5, None, None).unwrap(),
            payload: message,
        };

        let packet_bytes = packet.into_bytes();
        assert_eq!(
            Fragment::try_from_bytes(&packet_bytes),
            Err(ChunkingError::MalformedFragmentData)
        );
    }

    #[test]
    fn conversion_from_bytes_fails_for_too_short_fragmented_payload_if_not_last() {
        let mut rng = thread_rng();

        let mlen = UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 1;
        let mut message = vec![0u8; mlen];
        rng.fill_bytes(&mut message);

        let packet = Fragment {
            header: FragmentHeader::try_new_fragmented(12345, 10, 5, None, None).unwrap(),
            payload: message,
        };

        let packet_bytes = packet.into_bytes();
        assert_eq!(
            Fragment::try_from_bytes(&packet_bytes),
            Err(ChunkingError::MalformedFragmentData)
        );
    }

    #[test]
    fn can_be_converted_to_and_from_bytes_for_pre_linked_fragmented_payload() {
        let mut rng = thread_rng();

        let mut msg = vec![0u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN];
        rng.fill_bytes(&mut msg);

        let fragment = Fragment {
            header: FragmentHeader::try_new_fragmented(12345, 10, 1, Some(1234), None).unwrap(),
            payload: msg,
        };
        let packet_bytes = fragment.clone().into_bytes();
        assert_eq!(fragment, Fragment::try_from_bytes(&packet_bytes).unwrap());

        let mut msg = vec![0u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 20];
        rng.fill_bytes(&mut msg);

        let fragment = Fragment {
            header: FragmentHeader::try_new_fragmented(12345, 1, 1, Some(1234), None).unwrap(),
            payload: msg,
        };
        let packet_bytes = fragment.clone().into_bytes();
        assert_eq!(fragment, Fragment::try_from_bytes(&packet_bytes).unwrap());
    }

    #[test]
    fn conversion_from_bytes_fails_for_too_long_pre_linked_fragmented_payload() {
        let mut rng = thread_rng();

        let mlen = LINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 1;
        let mut message = vec![0u8; mlen];
        rng.fill_bytes(&mut message);

        let packet = Fragment {
            header: FragmentHeader::try_new_fragmented(12345, 10, 1, Some(1234), None).unwrap(),
            payload: message,
        };

        let packet_bytes = packet.into_bytes();
        assert_eq!(
            Fragment::try_from_bytes(&packet_bytes),
            Err(ChunkingError::MalformedFragmentData)
        );
    }

    #[test]
    fn can_be_converted_to_and_from_bytes_for_post_linked_fragmented_payload() {
        let mut rng = thread_rng();

        let mut msg = vec![0u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN];
        rng.fill_bytes(&mut msg);

        let fragment = Fragment {
            header: FragmentHeader::try_new_fragmented(
                12345,
                u8::max_value(),
                u8::max_value(),
                None,
                Some(1234),
            )
            .unwrap(),
            payload: msg,
        };
        let packet_bytes = fragment.clone().into_bytes();
        assert_eq!(fragment, Fragment::try_from_bytes(&packet_bytes).unwrap());

        let mut msg = vec![0u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 20];
        rng.fill_bytes(&mut msg);

        let fragment = Fragment {
            header: FragmentHeader::try_new_fragmented(
                12345,
                u8::max_value(),
                u8::max_value(),
                None,
                Some(1234),
            )
            .unwrap(),
            payload: msg,
        };
        let packet_bytes = fragment.clone().into_bytes();
        assert_eq!(fragment, Fragment::try_from_bytes(&packet_bytes).unwrap());
    }

    #[test]
    fn conversion_from_bytes_fails_for_too_long_post_linked_fragmented_payload() {
        let mut rng = thread_rng();

        let mlen = LINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 1;
        let mut message = vec![0u8; mlen];
        rng.fill_bytes(&mut message);

        let packet = Fragment {
            header: FragmentHeader::try_new_fragmented(
                12345,
                u8::max_value(),
                u8::max_value(),
                None,
                Some(1234),
            )
            .unwrap(),
            payload: message,
        };

        let packet_bytes = packet.into_bytes();
        assert_eq!(
            Fragment::try_from_bytes(&packet_bytes),
            Err(ChunkingError::MalformedFragmentData)
        );
    }

    #[test]
    fn unfragmented_fragment_can_be_created_with_payload_of_valid_length() {
        let payload = [1u8; UNFRAGMENTED_PAYLOAD_MAX_LEN];
        assert!(Fragment::try_new_unfragmented(&payload).is_ok());

        let payload = [1u8; UNFRAGMENTED_PAYLOAD_MAX_LEN - 1];
        assert!(Fragment::try_new_unfragmented(&payload).is_ok());

        let payload = [1u8; UNFRAGMENTED_PAYLOAD_MAX_LEN - 20];
        assert!(Fragment::try_new_unfragmented(&payload).is_ok());
    }

    #[test]
    fn unfragmented_fragment_returns_error_when_created_with_payload_of_invalid_length() {
        let payload = [1u8; UNFRAGMENTED_PAYLOAD_MAX_LEN + 1];
        assert!(Fragment::try_new_unfragmented(&payload).is_err());

        let payload = [1u8; UNFRAGMENTED_PAYLOAD_MAX_LEN + 20];
        assert!(Fragment::try_new_unfragmented(&payload).is_err());
    }

    #[test]
    fn unlinked_fragment_can_be_created_with_payload_of_valid_length() {
        let id = 12345;
        let full_payload = [1u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN];
        let non_full_payload = [1u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 1];
        let non_full_payload2 = [1u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 20];

        assert!(Fragment::try_new_fragmented(&full_payload, id, 10, 1, None, None).is_ok());
        assert!(Fragment::try_new_fragmented(&full_payload, id, 10, 5, None, None).is_ok());
        assert!(Fragment::try_new_fragmented(&full_payload, id, 10, 10, None, None).is_ok());
        assert!(Fragment::try_new_fragmented(&full_payload, id, 1, 1, None, None).is_ok());

        assert!(Fragment::try_new_fragmented(&non_full_payload, id, 10, 10, None, None).is_ok());
        assert!(Fragment::try_new_fragmented(&non_full_payload, id, 1, 1, None, None).is_ok());

        assert!(Fragment::try_new_fragmented(&non_full_payload2, id, 10, 10, None, None).is_ok());
        assert!(Fragment::try_new_fragmented(&non_full_payload2, id, 1, 1, None, None).is_ok());
    }

    #[test]
    fn unlinked_fragment_returns_error_when_created_with_payload_of_invalid_length() {
        let id = 12345;
        let non_full_payload = [1u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 1];
        let non_full_payload2 = [1u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 20];
        let too_much_payload = [1u8; UNLINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 1];

        assert!(Fragment::try_new_fragmented(&non_full_payload, id, 10, 1, None, None).is_err());
        assert!(Fragment::try_new_fragmented(&non_full_payload, id, 10, 5, None, None).is_err());

        assert!(Fragment::try_new_fragmented(&too_much_payload, id, 10, 1, None, None).is_err());
        assert!(Fragment::try_new_fragmented(&too_much_payload, id, 10, 5, None, None).is_err());
        assert!(Fragment::try_new_fragmented(&too_much_payload, id, 1, 1, None, None).is_err());

        assert!(Fragment::try_new_fragmented(&non_full_payload2, id, 10, 1, None, None).is_err());
        assert!(Fragment::try_new_fragmented(&non_full_payload2, id, 10, 5, None, None).is_err());
    }

    #[test]
    fn linked_fragment_can_be_created_with_payload_of_valid_length() {
        let id = 12345;
        let link_id = 1234;
        let full_payload = [1u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN];
        let non_full_payload = [1u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 1];
        let non_full_payload2 = [1u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 20];

        assert!(
            Fragment::try_new_fragmented(&full_payload, id, 10, 1, Some(link_id), None).is_ok()
        );
        assert!(Fragment::try_new_fragmented(&full_payload, id, 1, 1, Some(link_id), None).is_ok());
        assert!(
            Fragment::try_new_fragmented(&non_full_payload, id, 1, 1, Some(link_id), None).is_ok()
        );
        assert!(
            Fragment::try_new_fragmented(&non_full_payload2, id, 1, 1, Some(link_id), None).is_ok()
        );

        assert!(Fragment::try_new_fragmented(
            &full_payload,
            id,
            u8::max_value(),
            u8::max_value(),
            None,
            Some(link_id)
        )
        .is_ok());
    }

    #[test]
    fn linked_fragment_returns_error_when_created_with_payload_of_invalid_length() {
        let id = 12345;
        let link_id = 1234;
        let non_full_payload = [1u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 1];
        let non_full_payload2 = [1u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN - 20];
        let too_much_payload = [1u8; LINKED_FRAGMENTED_PAYLOAD_MAX_LEN + 1];

        assert!(
            Fragment::try_new_fragmented(&non_full_payload, id, 10, 1, Some(link_id), None)
                .is_err()
        );
        assert!(
            Fragment::try_new_fragmented(&non_full_payload2, id, 10, 1, Some(link_id), None)
                .is_err()
        );
        assert!(
            Fragment::try_new_fragmented(&too_much_payload, id, 10, 1, Some(link_id), None)
                .is_err()
        );
        assert!(
            Fragment::try_new_fragmented(&too_much_payload, id, 1, 1, Some(link_id), None).is_err()
        );

        assert!(Fragment::try_new_fragmented(
            &non_full_payload,
            id,
            u8::max_value(),
            u8::max_value(),
            None,
            Some(link_id)
        )
        .is_err());
        assert!(Fragment::try_new_fragmented(
            &non_full_payload2,
            id,
            u8::max_value(),
            u8::max_value(),
            None,
            Some(link_id)
        )
        .is_err());

        assert!(Fragment::try_new_fragmented(
            &too_much_payload,
            id,
            u8::max_value(),
            u8::max_value(),
            None,
            Some(link_id)
        )
        .is_err());
    }
}

#[cfg(test)]
mod fragment_header {
    use super::*;

    #[cfg(test)]
    mod unfragmented_payload {
        use super::*;

        #[test]
        fn can_be_converted_to_and_from_bytes_for_exact_number_of_bytes_provided() {
            let unfragmented_header = FragmentHeader::new_unfragmented();

            let header_bytes = unfragmented_header.to_bytes();
            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(unfragmented_header, recovered_header);
            assert_eq!(UNFRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn can_be_converted_to_and_from_bytes_for_more_than_required_number_of_bytes() {
            let unfragmented_header = FragmentHeader::new_unfragmented();

            let mut header_bytes = unfragmented_header.to_bytes();
            header_bytes.append(vec![1, 2, 3, 4, 5].as_mut());

            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(unfragmented_header, recovered_header);
            assert_eq!(UNFRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn retrieval_from_bytes_fail_for_empty_slice() {
            let empty_vec = Vec::new();

            assert!(FragmentHeader::try_from_bytes(&empty_vec).is_err())
        }

        #[test]
        fn retrieval_from_bytes_fail_for_invalid_fragmentation_flag() {
            let unfragmented_header = FragmentHeader::new_unfragmented();

            let mut header_bytes = unfragmented_header.to_bytes();
            header_bytes.append(vec![1, 2, 3, 4, 5].as_mut());

            // set the fragmentation flag
            header_bytes[0] |= 1 << 7;

            assert!(FragmentHeader::try_from_bytes(&header_bytes).is_err());
        }
    }

    #[cfg(test)]
    mod unlinked_fragmented_payload {
        use super::*;

        #[test]
        fn can_be_converted_to_and_from_bytes_for_exact_number_of_bytes_provided() {
            let fragmented_header =
                FragmentHeader::try_new_fragmented(12345, 10, 5, None, None).unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(UNLINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn can_be_converted_to_and_from_bytes_for_more_than_required_number_of_bytes() {
            let fragmented_header =
                FragmentHeader::try_new_fragmented(12345, 10, 5, None, None).unwrap();

            let mut header_bytes = fragmented_header.to_bytes();
            header_bytes.append(vec![1, 2, 3, 4, 5].as_mut());

            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(UNLINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn retrieval_from_bytes_fail_for_insufficient_number_of_bytes_provided() {
            let fragmented_header =
                FragmentHeader::try_new_fragmented(12345, 10, 5, None, None).unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let header_bytes = &header_bytes[..header_bytes.len() - 1];
            assert!(FragmentHeader::try_from_bytes(&header_bytes).is_err())
        }

        #[test]
        fn retrieval_from_bytes_fail_for_invalid_fragmentation_flag() {
            let fragmented_header =
                FragmentHeader::try_new_fragmented(10, 10, 5, None, None).unwrap();

            let mut header_bytes_low = fragmented_header.to_bytes();

            // clear the fragmentation flag
            header_bytes_low[0] &= !(1 << 7);

            let mut header_bytes_high = header_bytes_low.clone();
            // make sure first byte of id is non-empty (apart from the fragmentation flag)
            // note for anyone reading this test in the future: choice of '3' here is arbitrary.
            header_bytes_high[0] |= 1 << 3;

            // this will cause it to be parsed as 'unfragmented' header due to whole byte being set to 0
            // there isn't a really a good way of preventing that apart from adding even data overhead
            assert_eq!(
                FragmentHeader::new_unfragmented(),
                FragmentHeader::try_from_bytes(&header_bytes_low).unwrap().0
            );

            // however, this will have cause an error as there will be a value in the first byte
            assert!(FragmentHeader::try_from_bytes(&header_bytes_high).is_err());
        }

        #[test]
        fn retrieval_from_bytes_fail_for_invalid_link_flag() {
            let fragmented_header =
                FragmentHeader::try_new_fragmented(12345, 10, 5, None, None).unwrap();

            let mut header_bytes = fragmented_header.to_bytes();
            // set linked flag
            header_bytes[6] |= 1 << 7;
            assert!(FragmentHeader::try_from_bytes(&header_bytes).is_err());
        }

        #[test]
        fn creation_of_header_fails_if_current_fragment_is_higher_than_total() {
            assert!(FragmentHeader::try_new_fragmented(12345, 10, 11, None, None).is_err());
        }

        #[test]
        fn creation_of_header_fails_if_current_fragment_is_zero() {
            assert!(FragmentHeader::try_new_fragmented(12345, 10, 0, None, None).is_err());
        }

        #[test]
        fn creation_of_header_fails_if_total_fragments_is_zero() {
            assert!(FragmentHeader::try_new_fragmented(12345, 0, 0, None, None).is_err());
        }

        #[test]
        fn creation_of_header_fails_if_id_is_negative() {
            assert!(FragmentHeader::try_new_fragmented(-10, 10, 5, None, None).is_err());
        }

        #[test]
        fn fragmented_header_cannot_be_created_with_zero_id() {
            assert!(FragmentHeader::try_new_fragmented(0, 10, 5, None, None).is_err());
            assert!(FragmentHeader::try_new_fragmented(12345, 10, 5, Some(0), None).is_err());
            assert!(FragmentHeader::try_new_fragmented(
                12345,
                u8::max_value(),
                u8::max_value(),
                None,
                Some(0)
            )
            .is_err());
        }

        #[test]
        fn retrieval_from_bytes_fail_if_current_fragment_is_higher_than_total() {
            // manually create header to overwrite any constructor checks
            let header = FragmentHeader {
                is_fragmented: true,
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
                is_fragmented: true,
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
            assert!(FragmentHeader::try_new_fragmented(12345, 10, 1, Some(12345), None).is_err());
            assert!(FragmentHeader::try_new_fragmented(12345, 10, 10, None, Some(12345)).is_err());
        }

        #[test]
        fn can_only_be_pre_linked_for_first_fragment() {
            assert!(FragmentHeader::try_new_fragmented(12345, 10, 1, Some(1234), None).is_ok());
            assert!(FragmentHeader::try_new_fragmented(12345, 10, 2, Some(1234), None).is_err());
        }

        #[test]
        fn can_only_be_post_linked_for_last_fragment() {
            assert!(FragmentHeader::try_new_fragmented(12345, 10, 10, None, Some(1234)).is_ok());
            assert!(FragmentHeader::try_new_fragmented(
                12345,
                u8::max_value(),
                u8::max_value(),
                None,
                Some(1234),
            )
            .is_ok());
            assert!(FragmentHeader::try_new_fragmented(12345, 10, 2, Some(1234), None).is_err());
        }

        #[test]
        fn pre_linked_can_be_converted_to_and_from_bytes_for_exact_number_of_bytes_provided() {
            let fragmented_header =
                FragmentHeader::try_new_fragmented(12345, 10, 1, Some(1234), None).unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn pre_linked_can_be_converted_to_and_from_bytes_for_more_than_required_number_of_bytes() {
            let fragmented_header =
                FragmentHeader::try_new_fragmented(12345, 10, 1, Some(1234), None).unwrap();

            let mut header_bytes = fragmented_header.to_bytes();
            header_bytes.append(vec![1, 2, 3, 4, 5].as_mut());

            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn pre_linked_is_successfully_recovered_if_its_both_first_and_final_fragment() {
            let fragmented_header =
                FragmentHeader::try_new_fragmented(12345, 1, 1, Some(1234), None).unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn post_linked_can_be_converted_to_and_from_bytes_for_exact_number_of_bytes_provided() {
            let fragmented_header = FragmentHeader::try_new_fragmented(
                12345,
                u8::max_value(),
                u8::max_value(),
                None,
                Some(1234),
            )
            .unwrap();

            let header_bytes = fragmented_header.to_bytes();
            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }

        #[test]
        fn post_linked_can_be_converted_to_and_from_bytes_for_more_than_required_number_of_bytes() {
            let fragmented_header = FragmentHeader::try_new_fragmented(
                12345,
                u8::max_value(),
                u8::max_value(),
                None,
                Some(1234),
            )
            .unwrap();

            let mut header_bytes = fragmented_header.to_bytes();
            header_bytes.append(vec![1, 2, 3, 4, 5].as_mut());

            let (recovered_header, bytes_used) =
                FragmentHeader::try_from_bytes(&header_bytes).unwrap();
            assert_eq!(fragmented_header, recovered_header);
            assert_eq!(LINKED_FRAGMENTED_HEADER_LEN, bytes_used);
        }
    }
}
