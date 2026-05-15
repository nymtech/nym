// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    fragmentation::FragmentationError,
    packet::{
        LpFrame,
        frame::{LpFrameAttributes, LpFrameKind},
    },
};

#[derive(PartialEq, Clone, Debug)]
pub struct FragmentHeader {
    /// ID associated to this particular `Fragment`.
    id: u64,

    /// Total number of `Fragment`s,  used to be able to determine if entire
    /// set was fully received as well as to perform bound checks.
    total_fragments: u8,

    /// Since message is always fragmented into payloads of constant lengths
    /// (apart from possibly the last one), there's no need to use offsets like ipv4/ipv6
    /// and we can just simply enumerate the fragments to later reconstruct the message.
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
        // SAFETY : Three conversion from slices to arrays with correct size
        let total_fragments = value[8];
        let current_fragment = value[9];
        if current_fragment >= total_fragments {
            return Err(FragmentationError::FragmentIndexOutOfBounds);
        }

        Ok(FragmentHeader {
            #[allow(clippy::unwrap_used)]
            id: u64::from_be_bytes(value[0..8].try_into().unwrap()),
            total_fragments,
            current_fragment,
            #[allow(clippy::unwrap_used)]
            reserved: value[10..14].try_into().unwrap(),
        })
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Fragment {
    header: FragmentHeader,
    payload: Vec<u8>,
    frame_kind: LpFrameKind,
}

impl Fragment {
    // It's up to the caller to make sure values are valid
    fn new(
        payload: &[u8],
        id: u64,
        total_fragments: u8,
        current_fragment: u8,
        frame_kind: LpFrameKind,
    ) -> Self {
        let header = FragmentHeader::new(id, total_fragments, current_fragment);
        Fragment {
            header,
            payload: payload.to_vec(),
            frame_kind,
        }
    }

    pub fn into_lp_frame(self) -> LpFrame {
        LpFrame::new_with_attributes(self.frame_kind, self.header, self.payload)
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

    pub fn frame_kind(&self) -> LpFrameKind {
        self.frame_kind
    }

    /// Consumes `self` to obtain payload (i.e. part of original message) associated with this
    /// `Fragment`.
    pub(crate) fn extract_payload(self) -> Vec<u8> {
        self.payload
    }
}

impl TryFrom<LpFrame> for Fragment {
    type Error = FragmentationError;
    fn try_from(value: LpFrame) -> Result<Self, Self::Error> {
        if value.kind().is_fragmented() {
            Ok(Fragment {
                header: value.header.frame_attributes.try_into()?,
                payload: value.content.to_vec(),
                frame_kind: value.kind(),
            })
        } else {
            Err(FragmentationError::InvalidFrameKind)
        }
    }
}

/// Splits a payload into multiple `Fragment`s
/// This is meant to be used during Framing, not Chunking. This way we can ensure it fits in less than 255 fragments
pub fn fragment_payload<R: rand::Rng>(
    rng: &mut R,
    message: &[u8],
    frame_kind: LpFrameKind,
    fragment_payload_size: usize,
) -> Vec<Fragment> {
    debug_assert!(message.len() <= u8::MAX as usize * fragment_payload_size);
    debug_assert!(frame_kind.is_fragmented());

    let id = rng.r#gen();

    let num_fragments = (message.len() as f64 / fragment_payload_size as f64).ceil() as u8;

    let mut fragments = Vec::with_capacity(num_fragments as usize);

    for i in 0..num_fragments as usize {
        let lb = i * fragment_payload_size;
        let ub = usize::min(message.len(), (i + 1) * fragment_payload_size);
        fragments.push(Fragment::new(
            &message[lb..ub],
            id,
            num_fragments,
            i as u8,
            frame_kind,
        ))
    }

    fragments
}
