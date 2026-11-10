// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

pub mod fragment;
pub mod reconstruction;

#[derive(Debug, Error)]
pub enum FragmentationError {
    #[error("Fragment index is out of bounds for the announced length")]
    FragmentIndexOutOfBounds,

    #[error("Provided frame isn't fragmented")]
    InvalidFrameKind,

    #[error(
        "a {message_len} byte message needs {needed} fragments of {fragment_payload_size} bytes, more than the {} a message can have",
        u8::MAX
    )]
    TooManyFragments {
        message_len: usize,
        fragment_payload_size: usize,
        needed: usize,
    },

    #[error("a fragment payload size of zero cannot carry anything")]
    ZeroFragmentPayloadSize,
}
