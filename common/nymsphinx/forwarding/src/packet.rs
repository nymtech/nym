// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx_addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nym_sphinx_params::{PacketSize, PacketType, SphinxKeyRotation};
use nym_sphinx_types::{NymPacket, NymPacketError};

use nym_sphinx_anonymous_replies::reply_surb::AppliedReplySurb;
use nym_sphinx_params::key_rotation::InvalidSphinxKeyRotation;
use nym_sphinx_params::packet_sizes::InvalidPacketSize;
use nym_sphinx_params::packet_types::InvalidPacketType;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MixPacketFormattingError {
    #[error("too few bytes provided to recover from bytes")]
    TooFewBytesProvided,

    #[error("provided packet mode is invalid: {0}")]
    InvalidPacketType(#[from] InvalidPacketType),

    #[error("received request had an invalid packet size: {0}")]
    InvalidPacketSize(#[from] InvalidPacketSize),

    #[error("provided key rotation is invalid: {0}")]
    InvalidKeyRotation(#[from] InvalidSphinxKeyRotation),

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

#[derive(Debug)]
pub struct MixPacket {
    next_hop: NymNodeRoutingAddress,
    packet: NymPacket,
    packet_type: PacketType,
    key_rotation: SphinxKeyRotation,
}

impl MixPacket {
    pub fn new(
        next_hop: NymNodeRoutingAddress,
        packet: NymPacket,
        packet_type: PacketType,
        key_rotation: SphinxKeyRotation,
    ) -> Self {
        MixPacket {
            next_hop,
            packet,
            packet_type,
            key_rotation,
        }
    }

    pub fn from_applied_surb(
        applied_reply_surb: AppliedReplySurb,
        packet_type: PacketType,
    ) -> Self {
        MixPacket {
            next_hop: applied_reply_surb.first_hop_address(),
            key_rotation: applied_reply_surb.key_rotation(),
            packet: applied_reply_surb.into_packet(),
            packet_type,
        }
    }

    pub fn next_hop(&self) -> NymNodeRoutingAddress {
        self.next_hop
    }

    pub fn next_hop_address(&self) -> SocketAddr {
        self.next_hop.into()
    }

    pub fn packet(&self) -> &NymPacket {
        &self.packet
    }

    pub fn into_packet(self) -> NymPacket {
        self.packet
    }

    pub fn key_rotation(&self) -> SphinxKeyRotation {
        self.key_rotation
    }

    pub fn packet_type(&self) -> PacketType {
        self.packet_type
    }

    // the message is formatted as follows:
    // packet_type || FIRST_HOP || packet
    pub fn try_from_v1_bytes(b: &[u8]) -> Result<Self, MixPacketFormattingError> {
        // we need at least 1 byte to read packet type and another one to read type of the encoded first hop address
        if b.len() < 2 {
            return Err(MixPacketFormattingError::TooFewBytesProvided);
        }

        let packet_type = PacketType::try_from(b[0])?;

        let next_hop = NymNodeRoutingAddress::try_from_bytes(&b[1..])?;
        let addr_offset = next_hop.bytes_min_len();

        let packet_data = &b[addr_offset + 1..];
        let packet_size = packet_data.len();

        // make sure the received data length corresponds to a valid packet
        let _ = PacketSize::get_type(packet_size)?;

        let packet = match packet_type {
            PacketType::Mix => NymPacket::sphinx_from_bytes(packet_data)?,
            PacketType::Outfox => NymPacket::outfox_from_bytes(packet_data)?,
        };

        Ok(MixPacket {
            next_hop,
            packet,
            packet_type,
            key_rotation: SphinxKeyRotation::Unknown,
        })
    }

    pub fn into_v1_bytes(self) -> Result<Vec<u8>, MixPacketFormattingError> {
        Ok(std::iter::once(self.packet_type as u8)
            .chain(self.next_hop.as_bytes())
            .chain(self.packet.to_bytes()?)
            .collect())
    }

    // the message is formatted as follows:
    // packet_type || KEY_ROTATION || FIRST_HOP || packet
    pub fn try_from_v2_bytes(b: &[u8]) -> Result<Self, MixPacketFormattingError> {
        // we need at least 1 byte to read packet type, 1 byte to read key rotation
        // and finally another one to read type of the encoded first hop address
        if b.len() < 3 {
            return Err(MixPacketFormattingError::TooFewBytesProvided);
        }

        let packet_type = PacketType::try_from(b[0])?;
        let key_rotation = SphinxKeyRotation::try_from(b[1])?;

        let next_hop = NymNodeRoutingAddress::try_from_bytes(&b[2..])?;
        let addr_offset = next_hop.bytes_min_len();

        let packet_data = &b[addr_offset + 2..];
        let packet_size = packet_data.len();

        // make sure the received data length corresponds to a valid packet
        let _ = PacketSize::get_type(packet_size)?;

        let packet = match packet_type {
            PacketType::Mix => NymPacket::sphinx_from_bytes(packet_data)?,
            PacketType::Outfox => NymPacket::outfox_from_bytes(packet_data)?,
        };

        Ok(MixPacket {
            next_hop,
            packet,
            packet_type,
            key_rotation,
        })
    }

    pub fn into_v2_bytes(self) -> Result<Vec<u8>, MixPacketFormattingError> {
        Ok(std::iter::once(self.packet_type as u8)
            .chain(std::iter::once(self.key_rotation as u8))
            .chain(self.next_hop.as_bytes())
            .chain(self.packet.to_bytes()?)
            .collect())
    }
}

// TODO: test for serialization and errors!
