// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{array::TryFromSliceError, fmt};
use thiserror::Error;

#[cfg(feature = "sphinx")]
pub use sphinx_packet::{SphinxPacket, SphinxPacketBuilder};

#[cfg(feature = "sphinx")]
pub use sphinx_packet::{
    Error as SphinxError, ProcessedPacket, ProcessedPacketData,
    constants::{
        self, DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, MAX_PATH_LENGTH, NODE_ADDRESS_LENGTH,
        PAYLOAD_KEY_SIZE, REPLAY_TAG_SIZE,
    },
    crypto::{self, PrivateKey, PublicKey},
    header::{self, HEADER_SIZE, ProcessedHeader, SphinxHeader, delays, delays::Delay},
    packet::builder::DEFAULT_PAYLOAD_SIZE,
    payload::{
        PAYLOAD_OVERHEAD_SIZE, Payload,
        key::{PayloadKey, PayloadKeySeed, derive_payload_key},
    },
    route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, SURBIdentifier},
    surb::{SURB, SURBMaterial},
    version::*,
};

#[derive(Error, Debug)]
pub enum NymPacketError {
    #[error("Sphinx error: {0}")]
    #[cfg(feature = "sphinx")]
    Sphinx(#[from] sphinx_packet::Error),

    #[error("{0}")]
    FromSlice(#[from] TryFromSliceError),
}

// TODO: wrap that guy and add extra metadata to indicate key rotation?
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
pub enum NymPacket {
    #[cfg(feature = "sphinx")]
    Sphinx(SphinxPacket),
}

#[non_exhaustive]
pub enum NymProcessedPacket {
    #[cfg(feature = "sphinx")]
    Sphinx(ProcessedPacket),
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

    pub fn len(&self) -> usize {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sphinx")]
            NymPacket::Sphinx(packet) => packet.len(),
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
        }
    }

    #[cfg(feature = "sphinx")]
    #[allow(unreachable_patterns)]
    pub fn sphinx_packet_ref(&self) -> Option<&SphinxPacket> {
        match self {
            NymPacket::Sphinx(packet) => Some(packet),
            _ => None,
        }
    }

    #[cfg(feature = "sphinx")]
    #[allow(unreachable_patterns)]
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
