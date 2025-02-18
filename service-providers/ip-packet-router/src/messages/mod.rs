// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod request;
pub(crate) mod response;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ClientVersion {
    V7,
    V8,
}

impl ClientVersion {
    pub(crate) fn into_u8(self) -> u8 {
        match self {
            ClientVersion::V7 => 7,
            ClientVersion::V8 => 8,
        }
    }
}
