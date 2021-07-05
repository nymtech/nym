// Copyright 2021 Nym Technologies SA
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

use mixnet_contract::{GatewayBond, MixNodeBond};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use topology::{nym_topology_from_bonds, NymTopology};

#[derive(Deserialize)]
struct GoodTopology {
    mixnodes: Vec<MixNodeBond>,
    gateways: Vec<GatewayBond>,
}

pub(crate) fn parse_topology_file<P: AsRef<Path>>(file_path: P) -> NymTopology {
    let file_content =
        fs::read_to_string(file_path).expect("specified topology file does not exist");
    let good_topology = serde_json::from_str::<GoodTopology>(&file_content)
        .expect("topology in specified file is malformed");
    let nym_topology = nym_topology_from_bonds(good_topology.mixnodes, good_topology.gateways);
    if nym_topology.mixes().len() != 3 {
        panic!("topology has different than 3 number of layers")
    }
    if nym_topology.gateways().is_empty() {
        panic!("topology does not include a gateway")
    }

    nym_topology
}
