// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "outfox")]
pub use nym_outfox::{
    constants::MIN_PACKET_SIZE, constants::MIX_PARAMS_LEN, constants::OUTFOX_PACKET_OVERHEAD,
    error::OutfoxError,
};
// re-exporting types and constants available in sphinx
#[cfg(feature = "outfox")]
use nym_outfox::packet::{OutfoxPacket, OutfoxProcessedPacket};

use once_cell::sync::OnceCell;
#[cfg(feature = "sphinx")]
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
#[cfg(feature = "sphinx")]
use sphinx_packet::{SphinxPacket, SphinxPacketBuilder};
use std::{array::TryFromSliceError, fmt};
use thiserror::Error;

static REUSABLE_SURB: OnceCell<SURB> = OnceCell::new();
static REUSABLE_HEADER: OnceCell<ProcessedHeader> = OnceCell::new();

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
    pub fn from_surb<M: AsRef<[u8]>>(
        size: usize,
        message: M,
        route: &[Node],
        destination: &Destination,
        delays: &[Delay],
    ) -> Result<NymPacket, NymPacketError> {
        let (packet, _address) = if let Some(surb) = REUSABLE_SURB.get() {
            let new_surb = surb.clone();
            new_surb.use_surb(message.as_ref(), size)?
        } else {
            let surb_material =
                SURBMaterial::new(route.to_vec(), vec![Delay::new_from_millis(0)], destination.to_owned());
            let surb = SURB::new(EphemeralSecret::new(), surb_material)?;
            REUSABLE_SURB
                .set(surb.clone())
                .expect("ReusableSURB was already set!");
            surb.use_surb(message.as_ref(), size)?
        };

        Ok(NymPacket::Sphinx(packet))
    }

    #[cfg(feature = "sphinx")]
    pub fn sphinx_build<M: AsRef<[u8]>>(
        size: usize,
        message: M,
        route: &[Node],
        destination: &Destination,
        delays: &[Delay],
    ) -> Result<NymPacket, NymPacketError> {
        NymPacket::from_surb(size, message, route, destination, delays)
        // Ok(NymPacket::Sphinx(
        //     SphinxPacketBuilder::new()
        //         .with_payload_size(size)
        //         .build_packet(message, route, destination, delays)?,
        // ))
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
                let unwrapped_header: ProcessedHeader = if let Some(header) = REUSABLE_HEADER.get() {
                    header.clone()
                } else {
                    let header = packet.header.process(node_secret_key)?;
                    let _ = REUSABLE_HEADER.set(header.clone());
                    header
                };

                let processed_packet = match unwrapped_header {
                    ProcessedHeader::ForwardHop(
                        new_header,
                        next_hop_address,
                        delay,
                        payload_key,
                    ) => {
                        let new_payload = packet.payload.unwrap(&payload_key)?;
                        let new_packet = SphinxPacket {
                            header: *new_header,
                            payload: new_payload,
                        };
                        ProcessedPacket::ForwardHop(
                            Box::new(new_packet),
                            next_hop_address,
                            delay,
                        )
                    }
                    ProcessedHeader::FinalHop(destination, identifier, payload_key) => {
                        let new_payload = packet.payload.unwrap(&payload_key)?;
                        ProcessedPacket::FinalHop(
                            destination,
                            identifier,
                            new_payload,
                        )
                    }
                };

                Ok(NymProcessedPacket::Sphinx(processed_packet))
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
}
