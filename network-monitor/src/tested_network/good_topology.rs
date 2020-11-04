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

use std::fs;
use topology::NymTopology;
use validator_client::models::topology::Topology;

pub(crate) fn parse_topology_file(file_path: &str) -> NymTopology {
    let file_content =
        fs::read_to_string(file_path).expect("specified topology file does not exist");
    let validator_topology = serde_json::from_str::<Topology>(&file_content)
        .expect("topology in specified file is malformed");
    let nym_topology: NymTopology = validator_topology.into();
    if nym_topology.mixes().len() != 3 {
        panic!("topology has different than 3 number of layers")
    }
    if nym_topology.gateways().is_empty() {
        panic!("topology does not include a gateway")
    }

    nym_topology
}
