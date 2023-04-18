// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::identifier::prepare_identifier;
use crate::AckKey;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_addressing::nodes::{
    NymNodeRoutingAddress, NymNodeRoutingAddressError, MAX_NODE_ADDRESS_UNPADDED_LEN,
};
use nym_sphinx_params::packet_sizes::PacketSize;
use nym_sphinx_params::DEFAULT_NUM_MIX_HOPS;
use nym_sphinx_types::builder::SphinxPacketBuilder;
use nym_sphinx_types::SphinxError;
use nym_sphinx_types::{
    delays::{self, Delay},
    SphinxPacket,
};
use nym_topology::{NymTopology, NymTopologyError};
use rand::{CryptoRng, RngCore};
use std::convert::TryFrom;
use std::time;
use thiserror::Error;

pub struct SurbAck {
    surb_ack_packet: SphinxPacket,
    first_hop_address: NymNodeRoutingAddress,
    expected_total_delay: Delay,
}

#[derive(Debug, Error)]
pub enum SurbAckRecoveryError {
    #[error("received an invalid number of bytes to deserialize the SURB-Ack. Got {received}, expected {expected}")]
    InvalidPacketSize { received: usize, expected: usize },

    #[error("could not extract first hop address information - {0}")]
    InvalidAddress(#[from] NymNodeRoutingAddressError),

    #[error("the contained sphinx packet was not correctly formed - {0}")]
    InvalidSphinxPacket(#[from] SphinxError),
}

impl SurbAck {
    pub fn construct<R>(
        rng: &mut R,
        recipient: &Recipient,
        ack_key: &AckKey,
        marshaled_fragment_id: [u8; 5],
        average_delay: time::Duration,
        topology: &NymTopology,
    ) -> Result<Self, NymTopologyError>
    where
        R: RngCore + CryptoRng,
    {
        let route =
            topology.random_route_to_gateway(rng, DEFAULT_NUM_MIX_HOPS, recipient.gateway())?;
        let delays = delays::generate_from_average_duration(route.len(), average_delay);
        let destination = recipient.as_sphinx_destination();

        let surb_ack_payload = prepare_identifier(rng, ack_key, marshaled_fragment_id);

        let surb_ack_packet = SphinxPacketBuilder::new()
            .with_payload_size(PacketSize::AckPacket.payload_size())
            .build_packet(surb_ack_payload, &route, &destination, &delays)
            .unwrap();

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

    pub fn len() -> usize {
        // TODO: this will be variable once/if we decide to introduce optimization described
        // in common/nymsphinx/chunking/src/lib.rs:available_plaintext_size()
        PacketSize::AckPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN
    }

    pub fn expected_total_delay(&self) -> Delay {
        self.expected_total_delay
    }

    pub fn prepare_for_sending(self) -> (Delay, Vec<u8>) {
        // SURB_FIRST_HOP || SURB_ACK
        let surb_bytes: Vec<_> = self
            .first_hop_address
            .as_zero_padded_bytes(MAX_NODE_ADDRESS_UNPADDED_LEN)
            .into_iter()
            .chain(self.surb_ack_packet.to_bytes().into_iter())
            .collect();
        (self.expected_total_delay, surb_bytes)
    }

    // partial reciprocal of `prepare_for_sending` performed by the gateway
    pub fn try_recover_first_hop_packet(
        b: &[u8],
    ) -> Result<(NymNodeRoutingAddress, SphinxPacket), SurbAckRecoveryError> {
        if b.len() != Self::len() {
            Err(SurbAckRecoveryError::InvalidPacketSize {
                received: b.len(),
                expected: Self::len(),
            })
        } else {
            let address = NymNodeRoutingAddress::try_from_bytes(b)?;

            // TODO: this will be variable once/if we decide to introduce optimization described
            // in common/nymsphinx/chunking/src/lib.rs:available_plaintext_size()
            let address_offset = MAX_NODE_ADDRESS_UNPADDED_LEN;
            let packet = SphinxPacket::from_bytes(&b[address_offset..])?;

            Ok((address, packet))
        }
    }
}
