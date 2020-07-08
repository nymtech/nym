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
use nymsphinx_params::DEFAULT_NUM_MIX_HOPS;
use nymsphinx_types::{delays, Destination, Error as SphinxError, SURBMaterial, SURB};
use rand::{CryptoRng, RngCore};
use std::time;
use topology::{NymTopology, NymTopologyError};

pub struct ReplySURB(SURB);

#[derive(Debug)]
pub enum ReplySURBError {
    TooLongMessageError,
    RecoveryError(SphinxError),
}

impl ReplySURB {
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
        let destination = Destination::new(recipient.destination(), Default::default());

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

    pub fn use_surb(self, message: &[u8]) -> Result<Vec<u8>, ReplySURBError> {
        // SURB_FIRST_HOP || SURB_ACK

        // and here we have a roadblock. we have to have a 'destination' here which we, as a recipient,
        // do not know

        todo!()
    }
}
