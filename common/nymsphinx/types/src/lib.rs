// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(not(feature = "sphinx-only"))]
pub use nym_outfox::{
    constants::MIN_PACKET_SIZE, constants::MIX_PARAMS_LEN, constants::OUTFOX_PACKET_OVERHEAD,
    error::OutfoxError,
};
// re-exporting types and constants available in sphinx
#[cfg(not(feature = "sphinx-only"))]
use nym_outfox::packet::{OutfoxPacket, OutfoxProcessedPacket};
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
    #[cfg(not(feature = "sphinx-only"))]
    Outfox(#[from] nym_outfox::error::OutfoxError),

    #[error("{0}")]
    FromSlice(#[from] TryFromSliceError),
}

#[allow(clippy::large_enum_variant)]
pub enum NymPacket {
    Sphinx(SphinxPacket),
    #[cfg(not(feature = "sphinx-only"))]
    Outfox(OutfoxPacket),
}

pub enum NymProcessedPacket {
    Sphinx(ProcessedPacket),
    #[cfg(not(feature = "sphinx-only"))]
    Outfox(OutfoxProcessedPacket),
}

impl fmt::Debug for NymPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            NymPacket::Sphinx(packet) => f
                .debug_struct("NymPacket::Sphinx")
                .field("len", &packet.len())
                .finish(),
            #[cfg(not(feature = "sphinx-only"))]
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

    #[cfg(not(feature = "sphinx-only"))]
    pub fn outfox_build<M: AsRef<[u8]>>(
        payload: M,
        route: &[Node],
        destination: &Destination,
        size: Option<usize>,
    ) -> Result<NymPacket, NymPacketError> {
        Ok(NymPacket::Outfox(OutfoxPacket::build(
            payload,
            route.try_into()?,
            destination,
            size,
        )?))
    }

    #[cfg(not(feature = "sphinx-only"))]
    pub fn outfox_from_bytes(bytes: &[u8]) -> Result<NymPacket, NymPacketError> {
        Ok(NymPacket::Outfox(OutfoxPacket::try_from(bytes)?))
    }

    pub fn len(&self) -> usize {
        match self {
            NymPacket::Sphinx(packet) => packet.len(),
            #[cfg(not(feature = "sphinx-only"))]
            NymPacket::Outfox(packet) => packet.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, NymPacketError> {
        match self {
            NymPacket::Sphinx(packet) => Ok(packet.to_bytes()),
            #[cfg(not(feature = "sphinx-only"))]
            NymPacket::Outfox(packet) => Ok(packet.to_bytes()?),
        }
    }

    pub fn process(
        self,
        node_secret_key: &PrivateKey,
    ) -> Result<NymProcessedPacket, NymPacketError> {
        match self {
            NymPacket::Sphinx(packet) => {
                Ok(NymProcessedPacket::Sphinx(packet.process(node_secret_key)?))
            }
            #[cfg(not(feature = "sphinx-only"))]
            NymPacket::Outfox(mut packet) => {
                let next_address = packet.decode_next_layer(node_secret_key)?;
                Ok(NymProcessedPacket::Outfox(OutfoxProcessedPacket::new(
                    packet,
                    next_address,
                )))
            }
        }
    }
}
