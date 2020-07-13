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

use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::NymNodeRoutingAddress;
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_params::DEFAULT_NUM_MIX_HOPS;
use nymsphinx_types::{delays, Error as SphinxError, SURBMaterial, SphinxPacket, SURB};
use rand::{CryptoRng, RngCore};
use std::convert::TryFrom;
use std::time;
use topology::{NymTopology, NymTopologyError};

#[derive(Debug)]
pub enum ReplySURBError {
    TooLongMessageError,
    RecoveryError(SphinxError),
}

pub struct ReplySURB(SURB);

impl ReplySURB {
    // TODO: should this return `ReplySURBError` for consistency sake
    // or keep `NymTopologyError` because it's the only error it can actually return?
    pub fn construct<R>(
        rng: &mut R,
        recipient: &Recipient,
        average_delay: time::Duration,
        topology: &NymTopology,
    ) -> Result<Self, NymTopologyError>
    where
        R: RngCore + CryptoRng,
    {
        let route =
            topology.random_route_to_gateway(rng, DEFAULT_NUM_MIX_HOPS, &recipient.gateway())?;
        let delays = delays::generate_from_average_duration(route.len(), average_delay);
        let destination = recipient.as_sphinx_destination();

        let surb_material = SURBMaterial::new(route, delays, destination);

        // this can't fail as we know we have a valid route to gateway and have correct number of delays
        Ok(ReplySURB(surb_material.construct_SURB().unwrap()))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplySURBError> {
        let surb = match SURB::from_bytes(bytes) {
            Err(err) => return Err(ReplySURBError::RecoveryError(err)),
            Ok(surb) => surb,
        };
        Ok(ReplySURB(surb))
    }

    // Allows to optionally increase the packet size to send slightly longer reply.
    pub fn use_surb(
        self,
        message: &[u8],
        packet_size: Option<PacketSize>,
    ) -> Result<(SphinxPacket, NymNodeRoutingAddress), ReplySURBError> {
        let packet_size = packet_size.unwrap_or_else(|| Default::default());

        // there's no chunking in reply-surbs
        if message.len() > packet_size.plaintext_size() {
            return Err(ReplySURBError::TooLongMessageError);
        }

        // this can realistically only fail on too messages and we just checked for that
        let (packet, first_hop) = self
            .0
            .use_surb(message, packet_size.payload_size())
            .expect("this error indicates inconsistent message length checking - it shouldn't have happened!");

        let first_hop_address = NymNodeRoutingAddress::try_from(first_hop).unwrap();

        Ok((packet, first_hop_address))
    }
}
