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
pub struct FragmentHashKey(LpFrameKind, u64);

impl From<(LpFrameKind, u64)> for FragmentHashKey {
    fn from(value: (LpFrameKind, u64)) -> Self {
        FragmentHashKey(value.0, value.1)
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

    /// The kind of the inner frame
    next_frame_kind: LpFrameKind,

    reserved: [u8; 2],
}

impl FragmentHeader {
    // It's up to the caller to make sure values are valid
    fn new(
        id: u64,
        total_fragments: u8,
        current_fragment: u8,
        next_frame_kind: LpFrameKind,
    ) -> Self {
        FragmentHeader {
            id,
            total_fragments,
            current_fragment,
            next_frame_kind,
            reserved: [0; 2],
        }
    }
}

impl From<FragmentHeader> for LpFrameAttributes {
    fn from(value: FragmentHeader) -> Self {
        let mut buf = [0u8; 14];
        buf[0..8].copy_from_slice(&value.id.to_be_bytes());
        buf[8] = value.total_fragments;
        buf[9] = value.current_fragment;
        buf[10..12].copy_from_slice(&u16::to_be_bytes(value.next_frame_kind.into()));
        buf[12..14].copy_from_slice(&value.reserved);
        buf
    }
}

impl TryFrom<LpFrameAttributes> for FragmentHeader {
    type Error = FragmentationError;
    fn try_from(value: LpFrameAttributes) -> Result<Self, Self::Error> {
        let total_fragments = value[8];
        let current_fragment = value[9];
        if current_fragment >= total_fragments {
            return Err(FragmentationError::FragmentIndexOutOfBounds);
        }

        // SAFETY : Three conversion from slices to arrays with correct size
        Ok(FragmentHeader {
            #[allow(clippy::unwrap_used)]
            id: u64::from_be_bytes(value[0..8].try_into().unwrap()),
            total_fragments,
            current_fragment,
            #[allow(clippy::unwrap_used)]
            next_frame_kind: u16::from_be_bytes(value[10..12].try_into().unwrap()).into(),
            #[allow(clippy::unwrap_used)]
            reserved: value[12..14].try_into().unwrap(),
        })
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Fragment {
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
        next_frame_kind: LpFrameKind,
    ) -> Self {
        let header = FragmentHeader::new(id, total_fragments, current_fragment, next_frame_kind);
        Fragment {
            header,
            payload: payload.to_vec(),
        }
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

    pub fn next_frame_kind(&self) -> LpFrameKind {
        self.header.next_frame_kind
    }

    pub fn hash_key(&self) -> FragmentHashKey {
        (self.header.next_frame_kind, self.header.id).into()
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
        match value.kind() {
            LpFrameKind::FragmentedData => Ok(Fragment {
                header: value.header.frame_attributes.try_into()?,
                payload: value.content.to_vec(),
            }),
            _ => Err(FragmentationError::InvalidFrameKind),
        }
    }
}

/// Splits an LpFrame into multiple `Fragment`s
/// This is meant to be used during Framing, not Chunking. This way we can ensure it fits in less than 255 fragments
pub fn fragment_lp_message<R: rand::Rng>(
    rng: &mut R,
    message: LpFrame,
    fragment_payload_size: usize,
) -> Vec<Fragment> {
    debug_assert!(message.len() <= u8::MAX as usize * fragment_payload_size);

    let message_kind = message.kind();
    let message_bytes = message.to_bytes();

    let id = rng.r#gen();

    let num_fragments = (message_bytes.len() as f64 / fragment_payload_size as f64).ceil() as u8;

    let mut fragments = Vec::with_capacity(num_fragments as usize);

    for i in 0..num_fragments as usize {
        let lb = i * fragment_payload_size;
        let ub = usize::min(message_bytes.len(), (i + 1) * fragment_payload_size);
        fragments.push(Fragment::new(
            &message_bytes[lb..ub],
            id,
            num_fragments,
            i as u8,
            message_kind,
        ))
    }

    fragments
}
