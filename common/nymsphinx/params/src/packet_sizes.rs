// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::PacketType;
#[cfg(feature = "sphinx")]
use nym_sphinx_types::{header::HEADER_SIZE, PAYLOAD_OVERHEAD_SIZE};
#[cfg(feature = "outfox")]
use nym_sphinx_types::{MIN_PACKET_SIZE, MIX_PARAMS_LEN, OUTFOX_PACKET_OVERHEAD};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

// each sphinx packet contains mandatory header and payload padding + markers
#[cfg(feature = "sphinx")]
const SPHINX_PACKET_OVERHEAD: usize = HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE;

// it's up to the smart people to figure those values out : )

// TODO: even though we have 16B IV, is having just 5B (FRAG_ID_LEN) of the ID possibly insecure?

// TODO: I'm not entirely sure if we can easily extract `<AckEncryptionAlgorithm as NewStreamCipher>::NonceSize`
// into a const usize before relevant stuff is stabilised in rust...
#[cfg(feature = "sphinx")]
const ACK_IV_SIZE: usize = 16;

#[cfg(feature = "sphinx")]
const ACK_PACKET_SIZE: usize = ACK_IV_SIZE + crate::FRAG_ID_LEN + SPHINX_PACKET_OVERHEAD;
#[cfg(feature = "sphinx")]
const REGULAR_PACKET_SIZE: usize = 2 * 1024 + SPHINX_PACKET_OVERHEAD;
#[cfg(feature = "sphinx")]
const EXTENDED_PACKET_SIZE_8: usize = 8 * 1024 + SPHINX_PACKET_OVERHEAD;
#[cfg(feature = "sphinx")]
const EXTENDED_PACKET_SIZE_16: usize = 16 * 1024 + SPHINX_PACKET_OVERHEAD;
#[cfg(feature = "sphinx")]
const EXTENDED_PACKET_SIZE_32: usize = 32 * 1024 + SPHINX_PACKET_OVERHEAD;

#[cfg(feature = "outfox")]
const OUTFOX_ACK_PACKET_SIZE: usize = MIN_PACKET_SIZE + OUTFOX_PACKET_OVERHEAD;
#[cfg(feature = "outfox")]
const OUTFOX_REGULAR_PACKET_SIZE: usize = 2 * 1024 + OUTFOX_PACKET_OVERHEAD;

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
}

impl PartialOrd for PacketSize {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // order them by actual packet size
        Some(self.cmp(other))
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
            v => Err(InvalidPacketSize::UnknownPacketTag { received: v }),
        }
    }
}

impl PacketSize {
    pub const fn size(self) -> usize {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sphinx")]
            PacketSize::RegularPacket => REGULAR_PACKET_SIZE,
            #[cfg(feature = "sphinx")]
            PacketSize::AckPacket => ACK_PACKET_SIZE,
            #[cfg(feature = "sphinx")]
            PacketSize::ExtendedPacket8 => EXTENDED_PACKET_SIZE_8,
            #[cfg(feature = "sphinx")]
            PacketSize::ExtendedPacket16 => EXTENDED_PACKET_SIZE_16,
            #[cfg(feature = "sphinx")]
            PacketSize::ExtendedPacket32 => EXTENDED_PACKET_SIZE_32,
            #[cfg(feature = "outfox")]
            PacketSize::OutfoxRegularPacket => OUTFOX_REGULAR_PACKET_SIZE,
            #[cfg(feature = "outfox")]
            PacketSize::OutfoxAckPacket => OUTFOX_ACK_PACKET_SIZE,
            _ => 0,
        }
    }

    pub const fn header_size(&self) -> usize {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sphinx")]
            PacketSize::RegularPacket
            | PacketSize::AckPacket
            | PacketSize::ExtendedPacket8
            | PacketSize::ExtendedPacket16
            | PacketSize::ExtendedPacket32 => HEADER_SIZE,
            #[cfg(feature = "outfox")]
            PacketSize::OutfoxRegularPacket | PacketSize::OutfoxAckPacket => MIX_PARAMS_LEN,
            _ => 0,
        }
    }

    pub const fn payload_overhead(&self) -> usize {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sphinx")]
            PacketSize::RegularPacket
            | PacketSize::AckPacket
            | PacketSize::ExtendedPacket8
            | PacketSize::ExtendedPacket16
            | PacketSize::ExtendedPacket32 => PAYLOAD_OVERHEAD_SIZE,
            #[cfg(feature = "outfox")]
            PacketSize::OutfoxRegularPacket | PacketSize::OutfoxAckPacket => {
                OUTFOX_PACKET_OVERHEAD - MIX_PARAMS_LEN // Mix params are calculated into the total overhead so we take them out here
            }
            _ => 0,
        }
    }

    pub const fn plaintext_size(self) -> usize {
        self.size() - self.header_size() - self.payload_overhead()
    }

    pub const fn payload_size(self) -> usize {
        self.size() - self.header_size()
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
        } else if PacketSize::OutfoxRegularPacket.size() == size
            || PacketSize::OutfoxRegularPacket.size() == size + 6
        {
            Ok(PacketSize::OutfoxRegularPacket)
        } else if PacketSize::OutfoxAckPacket.size() == size {
            Ok(PacketSize::OutfoxAckPacket)
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

    pub fn get_type_from_plaintext(
        plaintext_size: usize,
        packet_type: PacketType,
    ) -> Result<Self, InvalidPacketSize> {
        #[allow(unreachable_patterns)]
        let overhead = match packet_type {
            #[cfg(feature = "sphinx")]
            PacketType::Mix => SPHINX_PACKET_OVERHEAD,
            #[allow(deprecated)]
            #[cfg(feature = "sphinx")]
            PacketType::Vpn => SPHINX_PACKET_OVERHEAD,
            #[cfg(feature = "outfox")]
            PacketType::Outfox => OUTFOX_PACKET_OVERHEAD,
            _ => 0,
        };
        let packet_size = plaintext_size + overhead;
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
