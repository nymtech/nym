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

use crate::models::gateway::RegisteredGateway;
use crate::models::mixnode::RegisteredMix;
use crate::models::validators::ValidatorsOutput;
use log::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use topology::{MixLayer, NymTopology};

// Topology shows us the current state of the overall Nym network
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub mix_nodes: Vec<RegisteredMix>,
    pub gateways: Vec<RegisteredGateway>,
    pub validators: ValidatorsOutput,
}

// changed from `TryInto`. reason being is that we should not fail entire topology
// conversion if there's one invalid node on the network screwing around
impl Into<NymTopology> for Topology {
    fn into(self) -> NymTopology {
        use std::collections::HashMap;

        let mut mixes = HashMap::new();
        for mix in self.mix_nodes.into_iter() {
            let layer = mix.mix_info.layer as MixLayer;
            if layer == 0 || layer > 3 {
                warn!(
                    "{} says it's on invalid layer {}!",
                    mix.mix_info.node_info.identity_key, layer
                );
                continue;
            }
            let mix_id = mix.mix_info.node_info.identity_key.clone();

            let layer_entry = mixes.entry(layer).or_insert(Vec::new());
            match mix.try_into() {
                Ok(mix) => layer_entry.push(mix),
                Err(err) => {
                    warn!("Mix {} is malformed - {:?}", mix_id, err);
                    continue;
                }
            }
        }

        let mut gateways = Vec::with_capacity(self.gateways.len());
        for gate in self.gateways.into_iter() {
            let gate_id = gate.gateway_info.node_info.identity_key.clone();
            match gate.try_into() {
                Ok(gate) => gateways.push(gate),
                Err(err) => {
                    warn!("Gateway {} is malformed - {:?}", gate_id, err);
                    continue;
                }
            }
        }

        NymTopology::new(mixes, gateways)
    }
}
