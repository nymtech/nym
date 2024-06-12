// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx_addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nym_sphinx_params::{PacketSize, PacketType};
use nym_sphinx_types::{NymPacket, NymPacketError};
use serde::{de::{self, Visitor}, Deserialize, Deserializer, Serialize, Serializer};

use std::fmt::{self, Debug, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MixPacketFormattingError {
    #[error("too few bytes provided to recover from bytes")]
    TooFewBytesProvided,
    #[error("provided packet mode is invalid")]
    InvalidPacketType,
    #[error("received request had invalid size - received {0}")]
    InvalidPacketSize(usize),
    #[error("address field was incorrectly encoded")]
    InvalidAddress,
    #[error("received sphinx packet was malformed")]
    MalformedSphinxPacket,
    #[error("Packet: {0}")]
    Packet(#[from] NymPacketError),
}

impl From<NymNodeRoutingAddressError> for MixPacketFormattingError {
    fn from(_: NymNodeRoutingAddressError) -> Self {
        MixPacketFormattingError::InvalidAddress
    }
}

pub struct MixPacket {
    next_hop: NymNodeRoutingAddress,
    packet: NymPacket,
    packet_type: PacketType,
}

impl Debug for MixPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MixPacket to {:?} with packet_type {:?}. Packet {:?}",
            self.next_hop, self.packet_type, self.packet
        )
    }
}

impl MixPacket {
    pub fn new(
        next_hop: NymNodeRoutingAddress,
        packet: NymPacket,
        packet_type: PacketType,
    ) -> Self {
        MixPacket {
            next_hop,
            packet,
            packet_type,
        }
    }

    pub fn next_hop(&self) -> NymNodeRoutingAddress {
        self.next_hop
    }

    pub fn packet(&self) -> &NymPacket {
        &self.packet
    }

    pub fn into_packet(self) -> NymPacket {
        self.packet
    }

    pub fn packet_type(&self) -> PacketType {
        self.packet_type
    }

    // the message is formatted as follows:
    // packet_type || FIRST_HOP || packet
    pub fn try_from_bytes(b: &[u8]) -> Result<Self, MixPacketFormattingError> {
        let packet_type = match PacketType::try_from(b[0]) {
            Ok(mode) => mode,
            Err(_) => return Err(MixPacketFormattingError::InvalidPacketType),
        };

        let next_hop = NymNodeRoutingAddress::try_from_bytes(&b[1..])?;
        let addr_offset = next_hop.bytes_min_len();

        let packet_data = &b[addr_offset + 1..];
        let packet_size = packet_data.len();
        if PacketSize::get_type(packet_size).is_err() {
            Err(MixPacketFormattingError::InvalidPacketSize(packet_size))
        } else {
            let packet = match packet_type {
                PacketType::Outfox => NymPacket::outfox_from_bytes(packet_data)?,
                _ => NymPacket::sphinx_from_bytes(packet_data)?,
            };

            Ok(MixPacket {
                next_hop,
                packet,
                packet_type,
            })
        }
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, MixPacketFormattingError> {
        Ok(std::iter::once(self.packet_type as u8)
            .chain(self.next_hop.as_bytes())
            .chain(self.packet.to_bytes()?)
            .collect())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, MixPacketFormattingError> {
        Ok(std::iter::once(self.packet_type as u8)
            .chain(self.next_hop.as_bytes())
            .chain(self.packet.to_bytes()?)
            .collect())
    }
}

impl Serialize for MixPacket {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.to_bytes().map_err(serde::ser::Error::custom)?)
    }
}

struct MixPacketVisitor;

impl <'de> Visitor<'de> for MixPacketVisitor {
    type Value = MixPacket;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a byte array representing a mix packet")
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        MixPacket::try_from_bytes(v).map_err(serde::de::Error::custom)
    }
}

impl <'de> Deserialize <'de> for MixPacket {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_bytes(MixPacketVisitor)
    }
}

// TODO: test for serialization and errors!
