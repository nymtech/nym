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

use crate::identifier::{prepare_identifier, AckAes128Key};
use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::{NymNodeRoutingAddress, MAX_NODE_ADDRESS_UNPADDED_LEN};
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_types::builder::SphinxPacketBuilder;
use nymsphinx_types::{
    delays::{self, Delay},
    Destination, SphinxPacket,
};
use rand::{CryptoRng, RngCore};
use std::convert::TryFrom;
use std::time;
use topology::{NymTopology, NymTopologyError};

#[allow(non_snake_case)]
pub struct SURBAck {
    surb_ack_packet: SphinxPacket,
    first_hop_address: NymNodeRoutingAddress,
    expected_total_delay: Delay,
}

#[derive(Debug)]
pub enum SURBAckRecoveryError {
    InvalidPacketSize,
    InvalidAddress,
    InvalidSphinxPacket,
}

impl SURBAck {
    pub fn construct<R, T>(
        rng: &mut R,
        recipient: &Recipient,
        ack_key: &AckAes128Key,
        marshaled_fragment_id: [u8; 5],
        average_delay: time::Duration,
        topology: &T,
    ) -> Result<Self, NymTopologyError>
    where
        R: RngCore + CryptoRng,
        T: NymTopology,
    {
        let route = topology.random_route_to_gateway(&recipient.gateway())?;
        let delays = delays::generate_from_average_duration(route.len(), average_delay);
        let destination = Destination::new(recipient.destination(), Default::default());

        let surb_ack_payload = prepare_identifier(rng, ack_key, marshaled_fragment_id);

        // once merged, that's an easy rng injection point for sphinx packets : )
        let surb_ack_packet = SphinxPacketBuilder::new()
            .with_payload_size(PacketSize::ACKPacket.payload_size())
            .build_packet(surb_ack_payload, &route, &destination, &delays)
            .unwrap();

        let expected_total_delay = delays.iter().sum();
        let first_hop_address =
            NymNodeRoutingAddress::try_from(route.first().unwrap().address.clone()).unwrap();

        Ok(SURBAck {
            surb_ack_packet,
            first_hop_address,
            expected_total_delay,
        })
    }

    pub fn len() -> usize {
        // TODO: this will be variable once/if we decide to introduce optimization described
        // in common/nymsphinx/chunking/src/lib.rs:available_plaintext_size()
        PacketSize::ACKPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN
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
    ) -> Result<(NymNodeRoutingAddress, SphinxPacket), SURBAckRecoveryError> {
        if b.len() != Self::len() {
            Err(SURBAckRecoveryError::InvalidPacketSize)
        } else {
            let address = match NymNodeRoutingAddress::try_from_bytes(&b) {
                Ok(address) => address,
                Err(_) => return Err(SURBAckRecoveryError::InvalidAddress),
            };

            // TODO: this will be variable once/if we decide to introduce optimization described
            // in common/nymsphinx/chunking/src/lib.rs:available_plaintext_size()
            let address_offset = MAX_NODE_ADDRESS_UNPADDED_LEN;
            let packet = match SphinxPacket::from_bytes(&b[address_offset..]) {
                Ok(packet) => packet,
                Err(_) => return Err(SURBAckRecoveryError::InvalidSphinxPacket),
            };

            Ok((address, packet))
        }
    }
}
