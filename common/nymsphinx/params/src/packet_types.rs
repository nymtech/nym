// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![allow(deprecated)]
// allow the u8 repr of `Vpn` PacketType whilst deprecating all of its other uses

use crate::PacketSize;
use serde::{Deserialize, Serialize};

use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("{received} is not a valid packet mode tag")]
pub struct InvalidPacketType {
    received: u8,
}

#[repr(u8)]
#[allow(deprecated)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PacketType {
    /// Represents 'normal' packet sent through the network that should be delayed by an appropriate
    /// value at each hop.
    #[default]
    #[serde(rename = "mix")]
    #[serde(alias = "sphinx")]
    Mix = 0,

    /// Represents a packet that should be sent through the network as fast as possible.
    #[deprecated]
    #[serde(rename = "unsupported-mix-vpn")]
    Vpn = 1,

    /// Abusing this to add Outfox support
    #[serde(rename = "outfox")]
    Outfox = 2,
}

impl fmt::Display for PacketType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PacketType::Mix => write!(f, "Mix"),
            #[allow(deprecated)]
            PacketType::Vpn => write!(f, "Vpn"),
            PacketType::Outfox => write!(f, "Outfox"),
        }
    }
}

impl PacketType {
    pub fn is_mix(self) -> bool {
        self == PacketType::Mix
    }

    pub fn is_outfox(self) -> bool {
        self == PacketType::Outfox
    }
}

impl TryFrom<u8> for PacketType {
    type Error = InvalidPacketType;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (PacketType::Mix as u8) => Ok(Self::Mix),
            _ if value == (PacketType::Outfox as u8) => Ok(Self::Outfox),
            v => Err(InvalidPacketType { received: v }),
        }
    }
}

impl From<PacketSize> for PacketType {
    fn from(s: PacketSize) -> Self {
        match s {
            PacketSize::RegularPacket => PacketType::Mix,
            PacketSize::AckPacket => PacketType::Mix,
            PacketSize::ExtendedPacket32 => PacketType::Mix,
            PacketSize::ExtendedPacket8 => PacketType::Mix,
            PacketSize::ExtendedPacket16 => PacketType::Mix,
            PacketSize::OutfoxRegularPacket => PacketType::Outfox,
            PacketSize::OutfoxAckPacket => PacketType::Outfox,
        }
    }
}
