// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{PacketSize, CURRENT_PACKET_VERSION_NUMBER};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PacketVersion {
    // this will allow updated mixnodes to still understand packets from before the update
    Legacy,
    Versioned(u8),
}

impl PacketVersion {
    pub fn new(use_legacy: bool) -> Self {
        if use_legacy {
            Self::new_legacy()
        } else {
            Self::new_versioned(CURRENT_PACKET_VERSION_NUMBER)
        }
    }

    pub fn new_legacy() -> Self {
        PacketVersion::Legacy
    }

    pub fn new_versioned(version: u8) -> Self {
        PacketVersion::Versioned(version)
    }

    pub fn is_legacy(&self) -> bool {
        matches!(self, PacketVersion::Legacy)
    }

    pub fn as_u8(&self) -> Option<u8> {
        match self {
            PacketVersion::Legacy => None,
            PacketVersion::Versioned(version) => Some(*version),
        }
    }
}

impl From<u8> for PacketVersion {
    fn from(v: u8) -> Self {
        match v {
            n if n == PacketSize::RegularPacket as u8 => PacketVersion::Legacy,
            n if n == PacketSize::AckPacket as u8 => PacketVersion::Legacy,
            n if n == PacketSize::ExtendedPacket8 as u8 => PacketVersion::Legacy,
            n if n == PacketSize::ExtendedPacket16 as u8 => PacketVersion::Legacy,
            n if n == PacketSize::ExtendedPacket32 as u8 => PacketVersion::Legacy,
            n => PacketVersion::Versioned(n),
        }
    }
}

impl Default for PacketVersion {
    fn default() -> Self {
        PacketVersion::Versioned(CURRENT_PACKET_VERSION_NUMBER)
    }
}
