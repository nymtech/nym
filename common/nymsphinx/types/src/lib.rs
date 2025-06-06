// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{array::TryFromSliceError, fmt};
use thiserror::Error;

#[cfg(feature = "outfox")]
pub use nym_outfox::packet::{OutfoxPacket, OutfoxProcessedPacket};

#[cfg(feature = "sphinx")]
pub use sphinx_packet::{SphinxPacket, SphinxPacketBuilder};

#[cfg(feature = "outfox")]
pub use nym_outfox::{
    constants::MIN_PACKET_SIZE, constants::MIX_PARAMS_LEN, constants::OUTFOX_PACKET_OVERHEAD,
    error::OutfoxError,
};
// re-exporting types and constants available in sphinx

#[cfg(feature = "sphinx")]
pub use sphinx_packet::{
    constants::{
        self, DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, MAX_PATH_LENGTH, NODE_ADDRESS_LENGTH,
        PAYLOAD_KEY_SIZE, REPLAY_TAG_SIZE,
    },
    crypto::{self, PrivateKey, PublicKey},
    header::{self, delays, delays::Delay, ProcessedHeader, SphinxHeader, HEADER_SIZE},
    packet::builder::DEFAULT_PAYLOAD_SIZE,
    payload::{
        key::{PayloadKey, PayloadKeySeed},
        Payload, PAYLOAD_OVERHEAD_SIZE,
    },
    route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, SURBIdentifier},
    surb::{SURBMaterial, SURB},
    version::*,
    Error as SphinxError, ProcessedPacket, ProcessedPacketData,
};

#[derive(Error, Debug)]
pub enum NymPacketError {
    #[error("Sphinx error: {0}")]
    #[cfg(feature = "sphinx")]
    Sphinx(#[from] sphinx_packet::Error),

    #[error("Outfox error: {0}")]
    #[cfg(feature = "outfox")]
    Outfox(#[from] nym_outfox::error::OutfoxError),

    #[error("{0}")]
    FromSlice(#[from] TryFromSliceError),
}

// TODO: wrap that guy and add extra metadata to indicate key rotation?
#[allow(clippy::large_enum_variant)]
pub enum NymPacket {
    #[cfg(feature = "sphinx")]
    Sphinx(SphinxPacket),
    #[cfg(feature = "outfox")]
    Outfox(OutfoxPacket),
}

pub enum NymProcessedPacket {
    #[cfg(feature = "sphinx")]
    Sphinx(ProcessedPacket),
    #[cfg(feature = "outfox")]
    Outfox(OutfoxProcessedPacket),
}

impl fmt::Debug for NymPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[allow(unreachable_patterns)]
        match &self {
            #[cfg(feature = "sphinx")]
            NymPacket::Sphinx(packet) => f
                .debug_struct("NymPacket::Sphinx")
                .field("len", &packet.len())
                .finish(),
            #[cfg(feature = "outfox")]
            NymPacket::Outfox(packet) => f
                .debug_struct("NymPacket::Outfox")
                .field("len", &packet.len())
                .finish(),
            _ => write!(f, ""),
        }
    }
}

impl NymPacket {
    #[cfg(feature = "sphinx")]
    pub fn sphinx_build<M: AsRef<[u8]>>(
        use_legacy_sphinx_format: bool,
        size: usize,
        message: M,
        route: &[Node],
        destination: &Destination,
        delays: &[Delay],
    ) -> Result<NymPacket, NymPacketError> {
        let mut builder = SphinxPacketBuilder::new().with_payload_size(size);

        if use_legacy_sphinx_format {
            builder = builder.with_version(X25519_WITH_EXPLICIT_PAYLOAD_KEYS_VERSION)
        };

        Ok(NymPacket::Sphinx(builder.build_packet(
            message,
            route,
            destination,
            delays,
        )?))
    }
    #[cfg(feature = "sphinx")]
    pub fn sphinx_from_bytes(bytes: &[u8]) -> Result<NymPacket, NymPacketError> {
        Ok(NymPacket::Sphinx(SphinxPacket::from_bytes(bytes)?))
    }

    #[cfg(feature = "outfox")]
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

    #[cfg(feature = "outfox")]
    pub fn outfox_from_bytes(bytes: &[u8]) -> Result<NymPacket, NymPacketError> {
        Ok(NymPacket::Outfox(OutfoxPacket::try_from(bytes)?))
    }

    pub fn len(&self) -> usize {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sphinx")]
            NymPacket::Sphinx(packet) => packet.len(),
            #[cfg(feature = "outfox")]
            NymPacket::Outfox(packet) => packet.len(),
            _ => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, NymPacketError> {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sphinx")]
            NymPacket::Sphinx(packet) => Ok(packet.to_bytes()),
            #[cfg(feature = "outfox")]
            NymPacket::Outfox(packet) => Ok(packet.to_bytes()?),
            _ => Ok(vec![]),
        }
    }

    #[cfg(feature = "sphinx")]
    pub fn process(
        self,
        node_secret_key: &PrivateKey,
    ) -> Result<NymProcessedPacket, NymPacketError> {
        match self {
            NymPacket::Sphinx(packet) => {
                Ok(NymProcessedPacket::Sphinx(packet.process(node_secret_key)?))
            }
            #[cfg(feature = "outfox")]
            NymPacket::Outfox(mut packet) => {
                let next_address = packet.decode_next_layer(node_secret_key)?;
                Ok(NymProcessedPacket::Outfox(OutfoxProcessedPacket::new(
                    packet,
                    next_address,
                )))
            }
        }
    }

    #[cfg(feature = "sphinx")]
    pub fn sphinx_packet_ref(&self) -> Option<&SphinxPacket> {
        match self {
            NymPacket::Sphinx(packet) => Some(packet),
            _ => None,
        }
    }

    #[cfg(feature = "sphinx")]
    pub fn to_sphinx_packet(self) -> Option<SphinxPacket> {
        match self {
            NymPacket::Sphinx(packet) => Some(packet),
            _ => None,
        }
    }

    #[cfg(feature = "sphinx")]
    pub fn is_sphinx(&self) -> bool {
        matches!(self, NymPacket::Sphinx(_))
    }
}
