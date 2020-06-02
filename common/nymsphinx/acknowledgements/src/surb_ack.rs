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
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_types::builder::SphinxPacketBuilder;
use nymsphinx_types::{
    delays::{self, Delay},
    Destination, SphinxPacket,
};
use rand::{CryptoRng, RngCore};
use std::time;
use topology::{NymTopology, NymTopologyError};

#[allow(non_snake_case)]
pub struct SURBAck {
    surb_ack_packet: SphinxPacket,
    expected_total_delay: Delay,
}

impl SURBAck {
    pub fn construct<R, T>(
        rng: &mut R,
        recipient: &Recipient,
        ack_key: &AckAes128Key,
        marshaled_fragment_id: &[u8],
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

        Ok(SURBAck {
            surb_ack_packet,
            expected_total_delay,
        })
    }

    pub fn prepare_for_sending(self) -> (Delay, Vec<u8>) {
        // TODO: once I make PR and change is merged, changed it to `into_bytes`
        (self.expected_total_delay, self.surb_ack_packet.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surb_packet_has_correct_size() {}
}
