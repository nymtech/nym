// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::FRAG_ID_LEN;
use nymsphinx_types::header::HEADER_SIZE;
use nymsphinx_types::PAYLOAD_OVERHEAD_SIZE;
use std::convert::TryFrom;
use std::str::FromStr;
use thiserror::Error;

// it's up to the smart people to figure those values out : )
const REGULAR_PACKET_SIZE: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE + 2 * 1024;
// TODO: even though we have 16B IV, is having just 5B (FRAG_ID_LEN) of the ID possibly insecure?

// TODO: I'm not entirely sure if we can easily extract `<AckEncryptionAlgorithm as NewStreamCipher>::NonceSize`
// into a const usize before relevant stuff is stabilised in rust...
const ACK_IV_SIZE: usize = 16;

const ACK_PACKET_SIZE: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE + ACK_IV_SIZE + FRAG_ID_LEN;
const EXTENDED_PACKET_SIZE_8: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE + 8 * 1024;
const EXTENDED_PACKET_SIZE_16: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE + 16 * 1024;
const EXTENDED_PACKET_SIZE_32: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE + 32 * 1024;

#[derive(Debug, Error)]
pub enum InvalidPacketSize {
    #[error("{received} is not a valid packet size tag")]
    UnknownPacketTag { received: u8 },

    #[error("{received} is not a valid extended packet size variant")]
    UnknownExtendedPacketVariant { received: String },

    #[error("{received} does not correspond with any known packet size")]
    UnknownPacketSize { received: usize },
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketSize {
    // for example instant messaging use case
    RegularPacket = 1,

    // for sending SURB-ACKs
    AckPacket = 2,

    // for example for streaming fast and furious in uncompressed 10bit 4K HDR quality
    ExtendedPacket32 = 3,

    // for example for streaming fast and furious in heavily compressed lossy RealPlayer quality
    ExtendedPacket8 = 4,

    // for example for streaming fast and furious in compressed XviD quality
    ExtendedPacket16 = 5,
}

impl FromStr for PacketSize {
    type Err = InvalidPacketSize;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "regular" => Ok(Self::RegularPacket),
            "ack" => Ok(Self::AckPacket),
            "extended8" => Ok(Self::ExtendedPacket8),
            "extended16" => Ok(Self::ExtendedPacket16),
            "extended32" => Ok(Self::ExtendedPacket32),
            s => Err(InvalidPacketSize::UnknownExtendedPacketVariant {
                received: s.to_string(),
            }),
        }
    }
}

impl TryFrom<u8> for PacketSize {
    type Error = InvalidPacketSize;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (PacketSize::RegularPacket as u8) => Ok(Self::RegularPacket),
            _ if value == (PacketSize::AckPacket as u8) => Ok(Self::AckPacket),
            _ if value == (PacketSize::ExtendedPacket8 as u8) => Ok(Self::ExtendedPacket8),
            _ if value == (PacketSize::ExtendedPacket16 as u8) => Ok(Self::ExtendedPacket16),
            _ if value == (PacketSize::ExtendedPacket32 as u8) => Ok(Self::ExtendedPacket32),
            v => Err(InvalidPacketSize::UnknownPacketTag { received: v }),
        }
    }
}

impl PacketSize {
    pub fn size(self) -> usize {
        match self {
            PacketSize::RegularPacket => REGULAR_PACKET_SIZE,
            PacketSize::AckPacket => ACK_PACKET_SIZE,
            PacketSize::ExtendedPacket8 => EXTENDED_PACKET_SIZE_8,
            PacketSize::ExtendedPacket16 => EXTENDED_PACKET_SIZE_16,
            PacketSize::ExtendedPacket32 => EXTENDED_PACKET_SIZE_32,
        }
    }

    pub fn plaintext_size(self) -> usize {
        self.size() - HEADER_SIZE - PAYLOAD_OVERHEAD_SIZE
    }

    pub fn payload_size(self) -> usize {
        self.size() - HEADER_SIZE
    }

    pub fn get_type(size: usize) -> Result<Self, InvalidPacketSize> {
        if PacketSize::RegularPacket.size() == size {
            Ok(PacketSize::RegularPacket)
        } else if PacketSize::AckPacket.size() == size {
            Ok(PacketSize::AckPacket)
        } else if PacketSize::ExtendedPacket8.size() == size {
            Ok(PacketSize::ExtendedPacket8)
        } else if PacketSize::ExtendedPacket16.size() == size {
            Ok(PacketSize::ExtendedPacket16)
        } else if PacketSize::ExtendedPacket32.size() == size {
            Ok(PacketSize::ExtendedPacket32)
        } else {
            Err(InvalidPacketSize::UnknownPacketSize { received: size })
        }
    }

    pub fn is_extended_size(&self) -> bool {
        match self {
            PacketSize::RegularPacket | PacketSize::AckPacket => false,
            PacketSize::ExtendedPacket8
            | PacketSize::ExtendedPacket16
            | PacketSize::ExtendedPacket32 => true,
        }
    }

    pub fn as_extended_size(self) -> Option<Self> {
        if self.is_extended_size() {
            Some(self)
        } else {
            None
        }
    }
}

impl Default for PacketSize {
    fn default() -> Self {
        PacketSize::RegularPacket
    }
}
