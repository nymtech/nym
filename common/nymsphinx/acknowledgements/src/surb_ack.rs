// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::identifier::prepare_identifier;
use crate::AckKey;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_addressing::nodes::{
    NymNodeRoutingAddress, NymNodeRoutingAddressError, MAX_NODE_ADDRESS_UNPADDED_LEN,
};
use nym_sphinx_params::packet_sizes::PacketSize;
use nym_sphinx_params::{PacketType, DEFAULT_NUM_MIX_HOPS};
use nym_sphinx_types::delays::Delay;
use nym_sphinx_types::{NymPacket, NymPacketError, MIN_PACKET_SIZE};
use nym_topology::{NymTopology, NymTopologyError};
use rand::{CryptoRng, RngCore};

use std::time;
use thiserror::Error;

#[derive(Debug)]
pub struct SurbAck {
    surb_ack_packet: NymPacket,
    first_hop_address: NymNodeRoutingAddress,
    expected_total_delay: Delay,
}

#[derive(Debug, Error)]
pub enum SurbAckRecoveryError {
    #[error("received an invalid number of bytes to deserialize the SURB-Ack. Got {received}, expected {expected}")]
    InvalidPacketSize { received: usize, expected: usize },

    #[error("could not extract first hop address information - {0}")]
    InvalidAddress(#[from] NymNodeRoutingAddressError),

    #[error("packet: {0}")]
    NymPacket(#[from] NymPacketError),
}

impl SurbAck {
    pub fn construct<R>(
        rng: &mut R,
        recipient: &Recipient,
        ack_key: &AckKey,
        marshaled_fragment_id: [u8; 5],
        average_delay: time::Duration,
        topology: &NymTopology,
        packet_type: PacketType,
    ) -> Result<Self, NymTopologyError>
    where
        R: RngCore + CryptoRng,
    {
        let route =
            topology.random_route_to_gateway(rng, DEFAULT_NUM_MIX_HOPS, recipient.gateway())?;
        let delays = nym_sphinx_routing::generate_hop_delays(average_delay, route.len());
        let destination = recipient.as_sphinx_destination();

        let surb_ack_payload = prepare_identifier(rng, ack_key, marshaled_fragment_id);
        let packet_size = match packet_type {
            PacketType::Outfox => surb_ack_payload.len().max(MIN_PACKET_SIZE),
            PacketType::Mix => PacketSize::AckPacket.payload_size(),
            #[allow(deprecated)]
            PacketType::Vpn => PacketSize::AckPacket.payload_size(),
        };

        let surb_ack_packet = match packet_type {
            PacketType::Outfox => NymPacket::outfox_build(
                surb_ack_payload,
                route.as_slice(),
                &destination,
                Some(packet_size),
            )?,
            PacketType::Mix => NymPacket::sphinx_build(
                packet_size,
                surb_ack_payload,
                &route,
                &destination,
                &delays,
            )?,
            #[allow(deprecated)]
            PacketType::Vpn => NymPacket::sphinx_build(
                packet_size,
                surb_ack_payload,
                &route,
                &destination,
                &delays,
            )?,
        };

        // in our case, the last hop is a gateway that does NOT do any delays
        let expected_total_delay = delays.iter().take(delays.len() - 1).sum();
        let first_hop_address =
            NymNodeRoutingAddress::try_from(route.first().unwrap().address).unwrap();

        Ok(SurbAck {
            surb_ack_packet,
            first_hop_address,
            expected_total_delay,
        })
    }

    pub fn len(packet_type: Option<PacketType>) -> usize {
        // TODO: this will be variable once/if we decide to introduce optimization described
        // in common/nymsphinx/chunking/src/lib.rs:available_plaintext_size()
        let packet_type = packet_type.unwrap_or(PacketType::Mix);
        match packet_type {
            PacketType::Outfox => {
                PacketSize::OutfoxAckPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN
            }
            PacketType::Mix => PacketSize::AckPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN,
            #[allow(deprecated)]
            PacketType::Vpn => PacketSize::AckPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN,
        }
    }

    pub fn expected_total_delay(&self) -> Delay {
        self.expected_total_delay
    }

    pub fn prepare_for_sending(self) -> Result<(Delay, Vec<u8>), SurbAckRecoveryError> {
        // SURB_FIRST_HOP || SURB_ACK
        let surb_bytes: Vec<_> = self
            .first_hop_address
            .as_zero_padded_bytes(MAX_NODE_ADDRESS_UNPADDED_LEN)
            .into_iter()
            .chain(self.surb_ack_packet.to_bytes()?)
            .collect();
        Ok((self.expected_total_delay, surb_bytes))
    }

    // partial reciprocal of `prepare_for_sending` performed by the gateway
    pub fn try_recover_first_hop_packet(
        b: &[u8],
        packet_type: PacketType,
    ) -> Result<(NymNodeRoutingAddress, NymPacket), SurbAckRecoveryError> {
        let address = NymNodeRoutingAddress::try_from_bytes(b)?;

        // TODO: this will be variable once/if we decide to introduce optimization described
        // in common/nymsphinx/chunking/src/lib.rs:available_plaintext_size()
        let address_offset = MAX_NODE_ADDRESS_UNPADDED_LEN;
        let packet = match packet_type {
            PacketType::Outfox => NymPacket::outfox_from_bytes(&b[address_offset..])?,
            PacketType::Mix => NymPacket::sphinx_from_bytes(&b[address_offset..])?,
            #[allow(deprecated)]
            PacketType::Vpn => NymPacket::sphinx_from_bytes(&b[address_offset..])?,
        };

        Ok((address, packet))
    }
}
