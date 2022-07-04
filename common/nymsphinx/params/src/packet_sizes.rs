// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketSize {
    // for example instant messaging use case
    RegularPacket = 1,

    // for sending SURB-ACKs
    AckPacket = 2,

    // for example for streaming fast and furious in uncompressed 10bit 4K HDR quality
    ExtendedPacket = 3,
}

impl TryFrom<u8> for PacketSize {
    type Error = InvalidPacketSize;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            _ if value == (PacketSize::RegularPacket as u8) => Ok(Self::RegularPacket),
            _ if value == (PacketSize::AckPacket as u8) => Ok(Self::AckPacket),
            _ if value == (PacketSize::ExtendedPacket as u8) => Ok(Self::ExtendedPacket),
            _ => Err(InvalidPacketSize),
        }
    }
}

impl PacketSize {
    pub fn size(self) -> usize {
        match self {
            PacketSize::RegularPacket => REGULAR_PACKET_SIZE,
            PacketSize::AckPacket => ACK_PACKET_SIZE,
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
        } else if PacketSize::AckPacket.size() == size {
            Ok(PacketSize::AckPacket)
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
