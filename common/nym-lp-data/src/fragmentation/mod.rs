// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

pub mod fragment;
pub mod reconstruction;

#[derive(Debug, Error)]
pub enum FragmentationError {
    #[error("Fragment index is out of bounds for the announced lentgh")]
    FragmentIndexOutOfBounds,

    #[error("Provided frame isn't fragmented")]
    InvalidFrameKind,
}
