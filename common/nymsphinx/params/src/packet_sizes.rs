// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::FRAG_ID_LEN;
use nym_sphinx_types::header::HEADER_SIZE;
use nym_sphinx_types::{OUTFOX_PACKET_OVERHEAD, PAYLOAD_OVERHEAD_SIZE};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

// each sphinx packet contains mandatory header and payload padding + markers
const SPHINX_PACKET_OVERHEAD: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE;

// it's up to the smart people to figure those values out : )

// TODO: even though we have 16B IV, is having just 5B (FRAG_ID_LEN) of the ID possibly insecure?

// TODO: I'm not entirely sure if we can easily extract `<AckEncryptionAlgorithm as NewStreamCipher>::NonceSize`
// into a const usize before relevant stuff is stabilised in rust...
const ACK_IV_SIZE: usize = 16;

const ACK_PACKET_SIZE: usize = ACK_IV_SIZE + FRAG_ID_LEN + SPHINX_PACKET_OVERHEAD;
const REGULAR_PACKET_SIZE: usize = 2 * 1024 + SPHINX_PACKET_OVERHEAD;
const EXTENDED_PACKET_SIZE_8: usize = 8 * 1024 + SPHINX_PACKET_OVERHEAD;
const EXTENDED_PACKET_SIZE_16: usize = 16 * 1024 + SPHINX_PACKET_OVERHEAD;
const EXTENDED_PACKET_SIZE_32: usize = 32 * 1024 + SPHINX_PACKET_OVERHEAD;

const OUTFOX_ACK_PACKET_SIZE: usize = ACK_IV_SIZE + FRAG_ID_LEN + OUTFOX_PACKET_OVERHEAD;
const OUTFOX_REGULAR_PACKET_SIZE: usize = 2 * 1024 + OUTFOX_PACKET_OVERHEAD;
const OUTFOX_EXTENDED_PACKET_SIZE_8: usize = 8 * 1024 + OUTFOX_PACKET_OVERHEAD;
const OUTFOX_EXTENDED_PACKET_SIZE_16: usize = 16 * 1024 + OUTFOX_PACKET_OVERHEAD;
const OUTFOX_EXTENDED_PACKET_SIZE_32: usize = 32 * 1024 + OUTFOX_PACKET_OVERHEAD;

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
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PacketSize {
    // for example instant messaging use case
    #[default]
    #[serde(rename = "regular")]
    RegularPacket = 1,

    // for sending SURB-ACKs
    #[serde(rename = "ack")]
    AckPacket = 2,

    // for example for streaming fast and furious in uncompressed 10bit 4K HDR quality
    #[serde(rename = "extended32")]
    ExtendedPacket32 = 3,

    // for example for streaming fast and furious in heavily compressed lossy RealPlayer quality
    #[serde(rename = "extended8")]
    ExtendedPacket8 = 4,

    // for example for streaming fast and furious in compressed XviD quality
    #[serde(rename = "extended16")]
    ExtendedPacket16 = 5,

    #[serde(rename = "outfox_regular")]
    OutfoxRegularPacket = 6,

    // for sending SURB-ACKs
    #[serde(rename = "outfox_ack")]
    OutfoxAckPacket = 7,

    // for example for streaming fast and furious in uncompressed 10bit 4K HDR quality
    #[serde(rename = "outfox_extended32")]
    OutfoxExtendedPacket32 = 8,

    // for example for streaming fast and furious in heavily compressed lossy RealPlayer quality
    #[serde(rename = "outfox_extended8")]
    OutfoxExtendedPacket8 = 9,

    // for example for streaming fast and furious in compressed XviD quality
    #[serde(rename = "outfox_extended16")]
    OutfoxExtendedPacket16 = 10,
}

impl PartialOrd for PacketSize {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // order them by actual packet size
        self.size().partial_cmp(&other.size())
    }
}

impl Ord for PacketSize {
    fn cmp(&self, other: &Self) -> Ordering {
        // order them by actual packet size
        self.size().cmp(&other.size())
    }
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
            "outfox_regular" => Ok(Self::OutfoxRegularPacket),
            "outfox_ack" => Ok(Self::OutfoxAckPacket),
            "outfox_extended8" => Ok(Self::OutfoxExtendedPacket8),
            "outfox_extended16" => Ok(Self::OutfoxExtendedPacket16),
            "outfox_extended32" => Ok(Self::OutfoxExtendedPacket32),
            s => Err(InvalidPacketSize::UnknownExtendedPacketVariant {
                received: s.to_string(),
            }),
        }
    }
}

impl Display for PacketSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketSize::RegularPacket => write!(f, "regular"),
            PacketSize::AckPacket => write!(f, "ack"),
            PacketSize::ExtendedPacket32 => write!(f, "extended32"),
            PacketSize::ExtendedPacket8 => write!(f, "extended8"),
            PacketSize::ExtendedPacket16 => write!(f, "extended16"),
            PacketSize::OutfoxRegularPacket => write!(f, "outfox_regular"),
            PacketSize::OutfoxAckPacket => write!(f, "outfox_ack"),
            PacketSize::OutfoxExtendedPacket32 => write!(f, "outfox_extended32"),
            PacketSize::OutfoxExtendedPacket8 => write!(f, "outfox_extended8"),
            PacketSize::OutfoxExtendedPacket16 => write!(f, "outfox_extended16"),
        }
    }
}

impl Debug for PacketSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = self.to_string();
        let size = self.size();
        let plaintext = self.plaintext_size();

        write!(f, "{name} ({size} bytes / {plaintext} plaintext)")
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
            _ if value == (PacketSize::OutfoxRegularPacket as u8) => Ok(Self::OutfoxRegularPacket),
            _ if value == (PacketSize::OutfoxAckPacket as u8) => Ok(Self::OutfoxAckPacket),
            _ if value == (PacketSize::OutfoxExtendedPacket8 as u8) => {
                Ok(Self::OutfoxExtendedPacket8)
            }
            _ if value == (PacketSize::OutfoxExtendedPacket16 as u8) => {
                Ok(Self::OutfoxExtendedPacket16)
            }
            _ if value == (PacketSize::OutfoxExtendedPacket32 as u8) => {
                Ok(Self::OutfoxExtendedPacket32)
            }
            v => Err(InvalidPacketSize::UnknownPacketTag { received: v }),
        }
    }
}

impl PacketSize {
    pub const fn size(self) -> usize {
        match self {
            PacketSize::RegularPacket => REGULAR_PACKET_SIZE,
            PacketSize::AckPacket => ACK_PACKET_SIZE,
            PacketSize::ExtendedPacket8 => EXTENDED_PACKET_SIZE_8,
            PacketSize::ExtendedPacket16 => EXTENDED_PACKET_SIZE_16,
            PacketSize::ExtendedPacket32 => EXTENDED_PACKET_SIZE_32,
            PacketSize::OutfoxRegularPacket => OUTFOX_REGULAR_PACKET_SIZE,
            PacketSize::OutfoxAckPacket => OUTFOX_ACK_PACKET_SIZE,
            PacketSize::OutfoxExtendedPacket8 => OUTFOX_EXTENDED_PACKET_SIZE_8,
            PacketSize::OutfoxExtendedPacket16 => OUTFOX_EXTENDED_PACKET_SIZE_16,
            PacketSize::OutfoxExtendedPacket32 => OUTFOX_EXTENDED_PACKET_SIZE_32,
        }
    }

    pub const fn plaintext_size(self) -> usize {
        self.size() - HEADER_SIZE - PAYLOAD_OVERHEAD_SIZE
    }

    pub const fn payload_size(self) -> usize {
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
            PacketSize::RegularPacket
            | PacketSize::AckPacket
            | PacketSize::OutfoxAckPacket
            | PacketSize::OutfoxRegularPacket => false,
            PacketSize::ExtendedPacket8
            | PacketSize::ExtendedPacket16
            | PacketSize::ExtendedPacket32
            | PacketSize::OutfoxExtendedPacket8
            | PacketSize::OutfoxExtendedPacket16
            | PacketSize::OutfoxExtendedPacket32 => true,
        }
    }

    pub fn as_extended_size(self) -> Option<Self> {
        if self.is_extended_size() {
            Some(self)
        } else {
            None
        }
    }

    pub fn get_type_from_plaintext(plaintext_size: usize) -> Result<Self, InvalidPacketSize> {
        let packet_size = plaintext_size + SPHINX_PACKET_OVERHEAD;
        Self::get_type(packet_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AckEncryptionAlgorithm;
    use nym_crypto::symmetric::stream_cipher::IvSizeUser;

    #[test]
    fn ack_iv_size_assertion() {
        let iv_size = AckEncryptionAlgorithm::iv_size();
        assert_eq!(iv_size, ACK_IV_SIZE);
    }
}
