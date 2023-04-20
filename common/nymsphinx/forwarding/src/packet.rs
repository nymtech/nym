// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx_addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nym_sphinx_params::{PacketMode, PacketSize};
use nym_sphinx_types::{
    NymPacket, NymPacketError, OutfoxError, OutfoxPacket, SphinxError, SphinxPacket,
};
use std::convert::TryFrom;
use std::fmt::{self, Debug, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MixPacketFormattingError {
    #[error("too few bytes provided to recover from bytes")]
    TooFewBytesProvided,
    #[error("provided packet mode is invalid")]
    InvalidPacketMode,
    #[error("received request had invalid size - received {0}")]
    InvalidPacketSize(usize),
    #[error("address field was incorrectly encoded")]
    InvalidAddress,
    #[error("received sphinx packet was malformed")]
    MalformedSphinxPacket,
    #[error("Outfox: {0}")]
    Outfox(#[from] OutfoxError),
    #[error("Sphinx: {0}")]
    Sphinx(#[from] SphinxError),
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
    packet_mode: PacketMode,
}

impl Debug for MixPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MixPacket to {:?} with packet_mode {:?}. Packet {:?}",
            self.next_hop, self.packet_mode, self.packet
        )
    }
}

impl MixPacket {
    pub fn new(
        next_hop: NymNodeRoutingAddress,
        packet: NymPacket,
        packet_mode: PacketMode,
    ) -> Self {
        MixPacket {
            next_hop,
            packet,
            packet_mode,
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

    pub fn packet_mode(&self) -> PacketMode {
        self.packet_mode
    }

    // the message is formatted as follows:
    // PACKET_MODE || FIRST_HOP || packet
    pub fn try_from_bytes(b: &[u8]) -> Result<Self, MixPacketFormattingError> {
        let packet_mode = match PacketMode::try_from(b[0]) {
            Ok(mode) => mode,
            Err(_) => return Err(MixPacketFormattingError::InvalidPacketMode),
        };

        let next_hop = NymNodeRoutingAddress::try_from_bytes(&b[1..])?;
        let addr_offset = next_hop.bytes_min_len();

        let packet_data = &b[addr_offset + 1..];
        let packet_size = packet_data.len();
        if PacketSize::get_type(packet_size).is_err() {
            Err(MixPacketFormattingError::InvalidPacketSize(packet_size))
        } else {
            let packet = match packet_mode {
                PacketMode::Outfox => NymPacket::Outfox(OutfoxPacket::try_from(packet_data)?),
                _ => NymPacket::Sphinx(SphinxPacket::from_bytes(packet_data)?),
            };

            Ok(MixPacket {
                next_hop,
                packet,
                packet_mode,
            })
        }
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, MixPacketFormattingError> {
        Ok(std::iter::once(self.packet_mode as u8)
            .chain(self.next_hop.as_bytes().into_iter())
            .chain(self.packet.to_bytes()?.into_iter())
            .collect())
    }
}

// TODO: test for serialization and errors!
