// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_outfox::{error::OutfoxError, packet::OutfoxPacket};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
// re-exporting types and constants available in sphinx
pub use sphinx_packet::{
    constants::{
        self, DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, MAX_PATH_LENGTH, NODE_ADDRESS_LENGTH,
        PAYLOAD_KEY_SIZE,
    },
    crypto::{self, EphemeralSecret, PrivateKey, PublicKey, SharedSecret},
    header::{self, delays, delays::Delay, ProcessedHeader, SphinxHeader, HEADER_SIZE},
    packet::builder::{self, DEFAULT_PAYLOAD_SIZE},
    payload::{Payload, PAYLOAD_OVERHEAD_SIZE},
    route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, SURBIdentifier},
    surb::{SURBMaterial, SURB},
    Error as SphinxError, ProcessedPacket, SphinxPacket,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NymPacketError {
    #[error("Sphinx error: {0}")]
    Sphinx(#[from] sphinx_packet::Error),

    #[error("Outfox error: {0}")]
    Outfox(#[from] nym_outfox::error::OutfoxError),
}

#[allow(clippy::large_enum_variant)]
pub enum NymPacket {
    Sphinx(SphinxPacket),
    Outfox(OutfoxPacket),
}

impl fmt::Debug for NymPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            NymPacket::Sphinx(packet) => f
                .debug_struct("NymPacket::Sphinx")
                .field("len", &packet.len())
                .finish(),
            NymPacket::Outfox(packet) => f
                .debug_struct("NymPacket::Outfox")
                .field("len", &packet.len())
                .finish(),
        }
    }
}

impl NymPacket {
    pub fn len(&self) -> usize {
        match self {
            NymPacket::Sphinx(packet) => packet.len(),
            NymPacket::Outfox(packet) => packet.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, NymPacketError> {
        match self {
            NymPacket::Sphinx(packet) => Ok(packet.to_bytes()),
            NymPacket::Outfox(packet) => Ok(packet.to_bytes()?),
        }
    }

    pub fn process(self, node_secret_key: &PrivateKey) -> Result<ProcessedPacket, NymPacketError> {
        match self {
            NymPacket::Sphinx(packet) => Ok(packet.process(node_secret_key)?),
            NymPacket::Outfox(_packet) => todo!(),
        }
    }
}

impl Serialize for NymPacket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.to_bytes().unwrap())
    }
}

use std::fmt;

use serde::de::{self, Visitor};

struct NymPacketVisitor;

impl<'de> Visitor<'de> for NymPacketVisitor {
    type Value = NymPacket;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Byte encoded NymPacket")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match SphinxPacket::from_bytes(v) {
            Ok(packet) => Ok(NymPacket::Sphinx(packet)),
            Err(_) => match OutfoxPacket::from_bytes(v) {
                Ok(packet) => Ok(NymPacket::Outfox(packet)),
                Err(_) => Err(E::custom(
                    "Could not deserialize Outfox nor Sphinx packet from bytes",
                )),
            },
        }
    }
}

impl<'de> Deserialize<'de> for NymPacket {
    fn deserialize<D>(deserializer: D) -> Result<NymPacket, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(NymPacketVisitor)
    }
}
