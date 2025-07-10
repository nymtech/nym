// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;

// wait, wait, but why are we starting with version 7?
// when packet header gets serialized, the following bytes (in that order) are put onto the wire:
// - packet_version (starting with v1.1.0)
// - packet_size indicator
// - packet_type
// - sphinx key rotation (starting with v1.13.0 - the Dolcelatte release)

// it also just so happens that the only valid values for packet_size indicator include values 1-6
// therefore if we receive byte `7` (or larger than that) we'll know we received a versioned packet,
// otherwise we should treat it as legacy
/// Increment it whenever we perform any breaking change in the wire format!
pub const INITIAL_PACKET_VERSION_NUMBER: u8 = 7;
pub const KEY_ROTATION_VERSION_NUMBER: u8 = 8;
pub const CURRENT_PACKET_VERSION_NUMBER: u8 = KEY_ROTATION_VERSION_NUMBER;
pub const CURRENT_PACKET_VERSION: PacketVersion =
    PacketVersion::unchecked(CURRENT_PACKET_VERSION_NUMBER);

pub const LEGACY_PACKET_VERSION: PacketVersion =
    PacketVersion::unchecked(INITIAL_PACKET_VERSION_NUMBER);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PacketVersion(u8);

impl Display for PacketVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Error)]
#[error("attempted to use legacy packet version")]
pub struct InvalidPacketVersion;

impl PacketVersion {
    pub fn new() -> Self {
        PacketVersion(CURRENT_PACKET_VERSION_NUMBER)
    }

    pub fn is_initial(&self) -> bool {
        self.0 == INITIAL_PACKET_VERSION_NUMBER
    }

    const fn unchecked(version: u8) -> PacketVersion {
        PacketVersion(version)
    }

    pub fn as_u8(&self) -> u8 {
        (*self).into()
    }
}

impl TryFrom<u8> for PacketVersion {
    type Error = InvalidPacketVersion;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < INITIAL_PACKET_VERSION_NUMBER {
            return Err(InvalidPacketVersion);
        }
        Ok(PacketVersion(value))
    }
}

impl From<PacketVersion> for u8 {
    fn from(packet_version: PacketVersion) -> Self {
        packet_version.0
    }
}

impl Default for PacketVersion {
    fn default() -> Self {
        PacketVersion::new()
    }
}
