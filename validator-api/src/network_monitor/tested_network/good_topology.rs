// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use mixnet_contract::{GatewayBond, MixNodeBond};
use serde::Deserialize;
use std::fs;
use topology::{nym_topology_from_bonds, NymTopology};

#[derive(Deserialize)]
struct GoodTopology {
    mixnodes: Vec<MixNodeBond>,
    gateways: Vec<GatewayBond>,
}

pub(crate) fn parse_topology_file(file_path: &str) -> NymTopology {
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
