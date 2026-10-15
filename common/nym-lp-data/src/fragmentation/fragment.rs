// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    fragmentation::FragmentationError,
    packet::{
        LpFrame,
        frame::{LpFrameAttributes, LpFrameHeader, LpFrameKind},
    },
};
use bytes::{Bytes, BytesMut};

#[derive(PartialEq, Clone, Debug)]
pub struct FragmentHeader {
    /// ID associated to this particular `Fragment`.
    id: u64,

    /// Total number of `Fragment`s,  used to be able to determine if entire
    /// set was fully received as well as to perform bound checks.
    total_fragments: u8,

    /// Index of this fragment, in (0..total_fragments)
    current_fragment: u8,

    reserved: [u8; 4],
}

impl FragmentHeader {
    // It's up to the caller to make sure values are valid
    fn new(id: u64, total_fragments: u8, current_fragment: u8) -> Self {
        FragmentHeader {
            id,
            total_fragments,
            current_fragment,
            reserved: [0; 4],
        }
    }
}

impl From<FragmentHeader> for LpFrameAttributes {
    fn from(value: FragmentHeader) -> Self {
        let mut buf = [0u8; 14];
        buf[0..8].copy_from_slice(&value.id.to_be_bytes());
        buf[8] = value.total_fragments;
        buf[9] = value.current_fragment;
        buf[10..14].copy_from_slice(&value.reserved);
        buf
    }
}

impl TryFrom<LpFrameAttributes> for FragmentHeader {
    type Error = FragmentationError;
    fn try_from(value: LpFrameAttributes) -> Result<Self, Self::Error> {
        let [id @ .., total_fragments, current_fragment, r0, r1, r2, r3] = value;
        if current_fragment >= total_fragments {
            return Err(FragmentationError::FragmentIndexOutOfBounds);
        }

        Ok(FragmentHeader {
            id: u64::from_be_bytes(id),
            total_fragments,
            current_fragment,
            reserved: [r0, r1, r2, r3],
        })
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Fragment {
    header: FragmentHeader,

    /// A view into the buffer the fragment came from, so splitting a message or receiving a
    /// fragment never copies its bytes.
    payload: Bytes,
}

impl Fragment {
    // It's up to the caller to make sure values are valid
    fn new(payload: Bytes, id: u64, total_fragments: u8, current_fragment: u8) -> Self {
        let header = FragmentHeader::new(id, total_fragments, current_fragment);
        Fragment { header, payload }
    }

    pub fn into_lp_frame(self) -> LpFrame {
        LpFrame::new_with_attributes(LpFrameKind::FragmentedData, self.header, self.payload)
    }

    /// Extracts id of this `Fragment`.
    pub fn id(&self) -> u64 {
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

    /// The part of the original message this `Fragment` carries.
    pub(crate) fn payload(&self) -> &[u8] {
        &self.payload
    }
}

impl TryFrom<LpFrame> for Fragment {
    type Error = FragmentationError;
    fn try_from(value: LpFrame) -> Result<Self, Self::Error> {
        match value.kind() {
            LpFrameKind::FragmentedData => Ok(Fragment {
                header: value.header.frame_attributes.try_into()?,
                payload: value.content,
            }),
            _ => Err(FragmentationError::InvalidFrameKind),
        }
    }
}

/// Splits an LpFrame into multiple `Fragment`s
/// This is meant to be used during Framing, not Chunking. This way we can ensure it fits in less than 255 fragments
///
/// Fails if the encoded message needs more fragments than a fragment header can count, or if
/// `fragment_payload_size` is zero.
pub fn fragment_lp_message<R: rand::Rng>(
    rng: &mut R,
    message: LpFrame,
    fragment_payload_size: usize,
) -> Result<Vec<Fragment>, FragmentationError> {
    if fragment_payload_size == 0 {
        return Err(FragmentationError::ZeroFragmentPayloadSize);
    }

    // encoded once; every fragment below is a view into this buffer
    let mut encoded = BytesMut::with_capacity(LpFrameHeader::SIZE + message.content.len());
    message.encode(&mut encoded);
    let encoded = encoded.freeze();

    let needed = encoded.len().div_ceil(fragment_payload_size);
    let num_fragments = u8::try_from(needed).map_err(|_| FragmentationError::TooManyFragments {
        message_len: encoded.len(),
        fragment_payload_size,
        needed,
    })?;

    let id = rng.next_u64();

    Ok((0..num_fragments)
        .map(|i| {
            let start = i as usize * fragment_payload_size;
            let end = usize::min(encoded.len(), start + fragment_payload_size);
            Fragment::new(encoded.slice(start..end), id, num_fragments, i)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// A message whose encoding, frame header included, is exactly `encoded_len` bytes.
    fn message_encoding_to(encoded_len: usize) -> LpFrame {
        LpFrame::new(
            LpFrameKind::SphinxPacket,
            vec![7u8; encoded_len - LpFrameHeader::SIZE],
        )
    }

    #[test]
    fn a_message_needing_the_most_fragments_a_header_can_count_is_split() {
        let fragments = fragment_lp_message(
            &mut StdRng::seed_from_u64(1),
            message_encoding_to(u8::MAX as usize * 10),
            10,
        )
        .unwrap();

        assert_eq!(fragments.len(), u8::MAX as usize);
        assert!(fragments.iter().all(|f| f.total_fragments() == u8::MAX));
        assert_eq!(fragments.last().unwrap().current_fragment(), u8::MAX - 1);
    }

    #[test]
    fn a_message_needing_one_fragment_more_is_rejected() {
        let result = fragment_lp_message(
            &mut StdRng::seed_from_u64(1),
            message_encoding_to(u8::MAX as usize * 10 + 1),
            10,
        );

        assert!(matches!(
            result,
            Err(FragmentationError::TooManyFragments { needed: 256, .. })
        ));
    }

    #[test]
    fn a_zero_fragment_payload_size_is_rejected() {
        let result = fragment_lp_message(&mut StdRng::seed_from_u64(1), message_encoding_to(32), 0);

        assert!(matches!(
            result,
            Err(FragmentationError::ZeroFragmentPayloadSize)
        ));
    }
}
