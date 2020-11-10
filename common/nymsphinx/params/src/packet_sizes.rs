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

use crate::FRAG_ID_LEN;
use nymsphinx_types::header::HEADER_SIZE;
use nymsphinx_types::PAYLOAD_OVERHEAD_SIZE;
use std::convert::TryFrom;

// it's up to the smart people to figure those values out : )
const REGULAR_PACKET_SIZE: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE + 2 * 1024;
// TODO: even though we have 16B IV, is having just 5B (FRAG_ID_LEN) of the ID possibly insecure?

// TODO: I'm not entirely sure if we can easily extract `<AckEncryptionAlgorithm as NewStreamCipher>::NonceSize`
// into a const usize before relevant stuff is stabilised in rust...
const ACK_IV_SIZE: usize = 16;

const ACK_PACKET_SIZE: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE + ACK_IV_SIZE + FRAG_ID_LEN;
const EXTENDED_PACKET_SIZE: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE + 32 * 1024;

#[derive(Debug)]
pub struct InvalidPacketSize;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PacketSize {
    // for example instant messaging use case
    RegularPacket = 1,

    // for sending SURB-ACKs
    ACKPacket = 2,

    // for example for streaming fast and furious in uncompressed 10bit 4K HDR quality
    ExtendedPacket = 3,
}

impl TryFrom<u8> for PacketSize {
    type Error = InvalidPacketSize;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            _ if value == (PacketSize::RegularPacket as u8) => Ok(Self::RegularPacket),
            _ if value == (PacketSize::ACKPacket as u8) => Ok(Self::ACKPacket),
            _ if value == (PacketSize::ExtendedPacket as u8) => Ok(Self::ExtendedPacket),
            _ => Err(InvalidPacketSize),
        }
    }
}

impl PacketSize {
    pub fn size(self) -> usize {
        match self {
            PacketSize::RegularPacket => REGULAR_PACKET_SIZE,
            PacketSize::ACKPacket => ACK_PACKET_SIZE,
            PacketSize::ExtendedPacket => EXTENDED_PACKET_SIZE,
        }
    }

    pub fn plaintext_size(self) -> usize {
        self.size() - HEADER_SIZE - PAYLOAD_OVERHEAD_SIZE
    }

    pub fn payload_size(self) -> usize {
        self.size() - HEADER_SIZE
    }

    pub fn get_type(size: usize) -> std::result::Result<Self, InvalidPacketSize> {
        if PacketSize::RegularPacket.size() == size {
            Ok(PacketSize::RegularPacket)
        } else if PacketSize::ACKPacket.size() == size {
            Ok(PacketSize::ACKPacket)
        } else if PacketSize::ExtendedPacket.size() == size {
            Ok(PacketSize::ExtendedPacket)
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
