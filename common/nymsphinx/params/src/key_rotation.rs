// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SphinxKeyRotation {
    // for legacy packets, where there's no explicit information which key has been used
    #[default]
    Unknown = 0,

    OddRotation = 1,

    EvenRotation = 2,
}

#[derive(Debug, Error)]
#[error("{received} is not a valid encoding of a sphinx key rotation")]
pub struct InvalidSphinxKeyRotation {
    received: u8,
}

// convert from particular rotation id into SphinxKeyRotation variant
impl From<u32> for SphinxKeyRotation {
    fn from(value: u32) -> Self {
        if value == 0 || value == u32::MAX {
            SphinxKeyRotation::Unknown
        } else if value % 2 == 0 {
            SphinxKeyRotation::EvenRotation
        } else {
            SphinxKeyRotation::OddRotation
        }
    }
}

// convert from an encoded SphinxKeyRotation into particular variant
// if value is actually provided, it MUST be one of the two. otherwise is invalid
impl TryFrom<u8> for SphinxKeyRotation {
    type Error = InvalidSphinxKeyRotation;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::Unknown as u8) => Ok(Self::Unknown),
            _ if value == (Self::OddRotation as u8) => Ok(Self::OddRotation),
            _ if value == (Self::EvenRotation as u8) => Ok(Self::EvenRotation),
            received => Err(InvalidSphinxKeyRotation { received }),
        }
    }
}
