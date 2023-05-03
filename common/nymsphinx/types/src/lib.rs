// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_outfox::{
    constants::MIX_PARAMS_LEN, constants::OUTFOX_PACKET_OVERHEAD, error::OutfoxError,
};
// re-exporting types and constants available in sphinx
use nym_outfox::packet::OutfoxPacket;
pub use sphinx_packet::{
    constants::{
        self, DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, MAX_PATH_LENGTH, NODE_ADDRESS_LENGTH,
        PAYLOAD_KEY_SIZE,
    },
    crypto::{self, EphemeralSecret, PrivateKey, PublicKey, SharedSecret},
    header::{self, delays, delays::Delay, ProcessedHeader, SphinxHeader, HEADER_SIZE},
    packet::builder::DEFAULT_PAYLOAD_SIZE,
    payload::{Payload, PAYLOAD_OVERHEAD_SIZE},
    route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, SURBIdentifier},
    surb::{SURBMaterial, SURB},
    Error as SphinxError, ProcessedPacket,
};
use sphinx_packet::{SphinxPacket, SphinxPacketBuilder};
use std::{array::TryFromSliceError, fmt};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NymPacketError {
    #[error("Sphinx error: {0}")]
    Sphinx(#[from] sphinx_packet::Error),

    #[error("Outfox error: {0}")]
    Outfox(#[from] nym_outfox::error::OutfoxError),

    #[error("{0}")]
    FromSlice(#[from] TryFromSliceError),
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
    pub fn sphinx_build<M: AsRef<[u8]>>(
        size: usize,
        message: M,
        route: &[Node],
        destination: &Destination,
        delays: &[Delay],
    ) -> Result<NymPacket, NymPacketError> {
        Ok(NymPacket::Sphinx(
            SphinxPacketBuilder::new()
                .with_payload_size(size)
                .build_packet(message, route, destination, delays)?,
        ))
    }
    pub fn sphinx_from_bytes(bytes: &[u8]) -> Result<NymPacket, NymPacketError> {
        Ok(NymPacket::Sphinx(SphinxPacket::from_bytes(bytes)?))
    }

    pub fn outfox_build<M: AsRef<[u8]>>(
        payload: M,
        route: &[Node],
        size: Option<usize>,
    ) -> Result<NymPacket, NymPacketError> {
        Ok(NymPacket::Outfox(OutfoxPacket::build(
            payload,
            route.try_into()?,
            size,
        )?))
    }

    pub fn outfox_from_bytes(bytes: &[u8]) -> Result<NymPacket, NymPacketError> {
        Ok(NymPacket::Outfox(OutfoxPacket::try_from(bytes)?))
    }

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
