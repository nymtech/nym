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

use nymsphinx_types::header::HEADER_SIZE;
use std::convert::TryFrom;

// it's up to the smart people to figure those values out : )
const REGULAR_PACKET_SIZE: usize = 2 * 1024;
const ACK_PACKET_SIZE: usize = 512;
const EXTENDED_PACKET_SIZE: usize = 32 * 1024;

pub struct InvalidPacketSize;

#[repr(u8)]
pub enum PacketSize {
    RegularPacket = 1,  // for example instant messaging use case
    ACKPacket = 2,      // for sending SURB-ACKs
    ExtendedPacket = 3, // for example for streaming fast and furious in uncompressed 10bit 4K HDR quality

    PreSURBChanges = 0,
}

impl TryFrom<u8> for PacketSize {
    type Error = InvalidPacketSize;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            _ if value == (PacketSize::RegularPacket as u8) => Ok(Self::RegularPacket),
            _ if value == (PacketSize::ACKPacket as u8) => Ok(Self::ACKPacket),
            _ if value == (PacketSize::ExtendedPacket as u8) => Ok(Self::ExtendedPacket),
            _ if value == (PacketSize::PreSURBChanges as u8) => Ok(Self::PreSURBChanges),
            _ => Err(InvalidPacketSize),
        }
    }
}

impl PacketSize {
    pub fn size(&self) -> usize {
        match &self {
            PacketSize::RegularPacket => REGULAR_PACKET_SIZE,
            PacketSize::ACKPacket => ACK_PACKET_SIZE,
            PacketSize::ExtendedPacket => EXTENDED_PACKET_SIZE,
            PacketSize::PreSURBChanges => nymsphinx_types::PACKET_SIZE,
        }
    }

    pub fn plaintext_size(&self) -> usize {
        self.size() - HEADER_SIZE // once merged also remove PAYLOAD OVERHEAD
    }

    pub fn get_type(size: usize) -> std::result::Result<Self, InvalidPacketSize> {
        if PacketSize::RegularPacket.size() == size {
            Ok(PacketSize::RegularPacket)
        } else if PacketSize::ACKPacket.size() == size {
            Ok(PacketSize::ACKPacket)
        } else if PacketSize::ExtendedPacket.size() == size {
            Ok(PacketSize::ExtendedPacket)
        } else if PacketSize::PreSURBChanges.size() == size {
            Ok(PacketSize::PreSURBChanges)
        } else {
            Err(InvalidPacketSize)
        }
    }
}

impl Default for PacketSize {
    fn default() -> Self {
        PacketSize::RegularPacket
    }
}
