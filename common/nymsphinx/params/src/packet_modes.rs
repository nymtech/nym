// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::convert::TryFrom;

#[derive(Debug)]
pub struct InvalidPacketMode;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
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

    pub fn is_vpn(self) -> bool {
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
