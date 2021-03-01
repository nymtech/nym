// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::identifier::prepare_identifier;
use crate::AckKey;
use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::{NymNodeRoutingAddress, MAX_NODE_ADDRESS_UNPADDED_LEN};
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_params::DEFAULT_NUM_MIX_HOPS;
use nymsphinx_types::builder::SphinxPacketBuilder;
use nymsphinx_types::{
    delays::{self, Delay},
    EphemeralSecret, SphinxPacket,
};
use rand::{CryptoRng, RngCore};
use std::convert::TryFrom;
use std::time;
use topology::{NymTopology, NymTopologyError};

pub struct SurbAck {
    surb_ack_packet: SphinxPacket,
    first_hop_address: NymNodeRoutingAddress,
    expected_total_delay: Delay,
}

#[derive(Debug)]
pub enum SurbAckRecoveryError {
    InvalidPacketSize,
    InvalidAddress,
    InvalidSphinxPacket,
}

impl SurbAck {
    pub fn construct<R>(
        rng: &mut R,
        recipient: &Recipient,
        ack_key: &AckKey,
        marshaled_fragment_id: [u8; 5],
        average_delay: time::Duration,
        topology: &NymTopology,
        initial_sphinx_secret: Option<&EphemeralSecret>,
    ) -> Result<Self, NymTopologyError>
    where
        R: RngCore + CryptoRng,
    {
        let route =
            topology.random_route_to_gateway(rng, DEFAULT_NUM_MIX_HOPS, &recipient.gateway())?;
        let delays = delays::generate_from_average_duration(route.len(), average_delay);
        let destination = recipient.as_sphinx_destination();

        let surb_ack_payload = prepare_identifier(rng, ack_key, marshaled_fragment_id);

        let mut surb_builder =
            SphinxPacketBuilder::new().with_payload_size(PacketSize::AckPacket.payload_size());
        if let Some(initial_secret) = initial_sphinx_secret {
            surb_builder = surb_builder.with_initial_secret(initial_secret);
        }

        let surb_ack_packet = surb_builder
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
            Err(SurbAckRecoveryError::InvalidPacketSize)
        } else {
            let address = match NymNodeRoutingAddress::try_from_bytes(&b) {
                Ok(address) => address,
                Err(_) => return Err(SurbAckRecoveryError::InvalidAddress),
            };

            // TODO: this will be variable once/if we decide to introduce optimization described
            // in common/nymsphinx/chunking/src/lib.rs:available_plaintext_size()
            let address_offset = MAX_NODE_ADDRESS_UNPADDED_LEN;
            let packet = match SphinxPacket::from_bytes(&b[address_offset..]) {
                Ok(packet) => packet,
                Err(_) => return Err(SurbAckRecoveryError::InvalidSphinxPacket),
            };

            Ok((address, packet))
        }
    }
}
