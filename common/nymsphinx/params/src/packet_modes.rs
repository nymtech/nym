// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;

#[derive(Debug)]
pub struct InvalidPacketMode;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketMode {
    /// Represents 'normal' packet sent through the network that should be delayed by an appropriate
    /// value at each hop.
    Mix = 0,

    /// Represents a VPN packet that should not be delayed and ideally cached pre-computed keys
    /// should be used for unwrapping data. Note that it does not offer the same level of anonymity.
    Vpn = 1,
}

impl PacketMode {
    pub fn is_mix(self) -> bool {
        self == PacketMode::Mix
    }

    pub fn is_old_vpn(self) -> bool {
        self == PacketMode::Vpn
    }
}

impl TryFrom<u8> for PacketMode {
    type Error = InvalidPacketMode;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            _ if value == (PacketMode::Mix as u8) => Ok(Self::Mix),
            _ if value == (PacketMode::Vpn as u8) => Ok(Self::Vpn),
            _ => Err(InvalidPacketMode),
        }
    }
}

impl Default for PacketMode {
    fn default() -> Self {
        PacketMode::Mix
    }
}
