// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
