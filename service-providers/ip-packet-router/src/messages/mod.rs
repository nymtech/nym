// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(crate) mod request;
pub(crate) mod response;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ClientVersion {
    V8,
    V9,
    V10,
}

impl ClientVersion {
    pub(crate) fn into_u8(self) -> u8 {
        match self {
            ClientVersion::V8 => 8,
            ClientVersion::V9 => 9,
            ClientVersion::V10 => 10,
        }
    }
}
