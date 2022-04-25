// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoconutInterfaceError {
    #[error("not enough bytes: {0} received, minimum {1} required")]
    InvalidByteLength(usize, usize),

    #[error("Could not decode base 58 string - {0}")]
    MalformedString(#[from] bs58::decode::Error),

    #[error("Not enough public attributes were specified")]
    NotEnoughPublicAttributes,

    #[error("Could not recover bandwidth value")]
    InvalidBandwidth,
}
