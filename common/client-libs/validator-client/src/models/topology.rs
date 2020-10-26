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
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use topology::NymTopology;

#[derive(Debug)]
pub enum TopologyConversionError {}

// Topology shows us the current state of the overall Nym network
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    mix_nodes: Vec<RegisteredMix>,
    gateways: Vec<RegisteredGateway>,
}

impl TryInto<NymTopology> for Topology {
    type Error = TopologyConversionError;

    fn try_into(self) -> Result<NymTopology, Self::Error> {
        unimplemented!()
    }
}
