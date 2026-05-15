// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    fragmentation::FragmentationError,
    packet::{
        LpFrame,
        frame::{LpFrameAttributes, LpFrameKind},
    },
};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
/// Key for reconstruction hashmap
pub struct FragmentHashKey(u64, LpFrameKind);

impl From<(u64, LpFrameKind)> for FragmentHashKey {
    fn from(value: (u64, LpFrameKind)) -> Self {
        FragmentHashKey(value.0, value.1)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FragmentMetadata(LpFrameKind, [u8; 4]);

impl FragmentMetadata {
    pub fn kind(&self) -> LpFrameKind {
        self.0
    }

    pub fn metadata(&self) -> [u8; 4] {
        self.1
    }
}
impl From<(LpFrameKind, [u8; 4])> for FragmentMetadata {
    fn from(value: (LpFrameKind, [u8; 4])) -> Self {
        FragmentMetadata(value.0, value.1)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct FragmentHeader {
    /// ID associated to this particular `Fragment`.
    id: u64,

    /// Total number of `Fragment`s,  used to be able to determine if entire
    /// set was fully received as well as to perform bound checks.
    total_fragments: u8,

    /// Index of this fragment, in (0..total_fragments)
    current_fragment: u8,

    /// Additional metadata, parsing depends on the frame_kind of the fragment.
    kind_metadata: [u8; 4],
}

impl FragmentHeader {
    // It's up to the caller to make sure values are valid
    fn new(id: u64, total_fragments: u8, current_fragment: u8, kind_metadata: [u8; 4]) -> Self {
        FragmentHeader {
            id,
            total_fragments,
            current_fragment,
            kind_metadata,
        }
    }
}

impl From<FragmentHeader> for LpFrameAttributes {
    fn from(value: FragmentHeader) -> Self {
        let mut buf = [0u8; 14];
        buf[0..8].copy_from_slice(&value.id.to_be_bytes());
        buf[8] = value.total_fragments;
        buf[9] = value.current_fragment;
        buf[10..14].copy_from_slice(&value.kind_metadata);
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
            kind_metadata: value[10..14].try_into().unwrap(),
        })
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Fragment {
    frame_kind: LpFrameKind,
    header: FragmentHeader,
    payload: Vec<u8>,
}

impl Fragment {
    // It's up to the caller to make sure values are valid
    fn new(
        payload: &[u8],
        id: u64,
        total_fragments: u8,
        current_fragment: u8,
        kind_metadata: [u8; 4],
        frame_kind: LpFrameKind,
    ) -> Self {
        let header = FragmentHeader::new(id, total_fragments, current_fragment, kind_metadata);
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

    pub fn kind_metadata(&self) -> [u8; 4] {
        self.header.kind_metadata
    }

    pub fn hash_key(&self) -> FragmentHashKey {
        (self.header.id, self.frame_kind).into()
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
    fragment_metadata: FragmentMetadata,
    fragment_payload_size: usize,
) -> Vec<Fragment> {
    debug_assert!(message.len() <= u8::MAX as usize * fragment_payload_size);
    debug_assert!(fragment_metadata.kind().is_fragmented());

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
            fragment_metadata.metadata(),
            fragment_metadata.kind(),
        ))
    }

    fragments
}
