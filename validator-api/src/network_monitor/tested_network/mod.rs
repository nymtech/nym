// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use mixnet_contract::{GatewayBond, MixNodeBond};
use topology::{gateway, mix, NymTopology};

pub(crate) mod good_topology;

#[derive(Clone)]
pub(crate) struct TestedNetwork {
    system_version: String,
    good_v4_topology: NymTopology,
    good_v6_topology: NymTopology,
}

impl TestedNetwork {
    pub(crate) fn new_good(good_v4_topology: NymTopology, good_v6_topology: NymTopology) -> Self {
        TestedNetwork {
            system_version: env!("CARGO_PKG_VERSION").to_owned(),
            good_v4_topology,
            good_v6_topology,
        }
    }

    pub(crate) fn main_v4_gateway(&self) -> &gateway::Node {
        if self.good_v4_topology.gateways().len() > 1 {
            warn!("we have more than a single 'good' gateway and in few places we made assumptions that only a single one existed!")
        }

        self.good_v4_topology
            .gateways()
            .get(0)
            .expect("our good v4 topology does not have any gateway specified!")
    }

    pub(crate) fn system_version(&self) -> &str {
        &self.system_version
    }

    pub(crate) fn substitute_mix(&self, node: mix::Node) -> NymTopology {
        let mut good_topology = self.good_v4_topology.clone();

        good_topology.set_mixes_in_layer(node.layer as u8, vec![node]);
        good_topology
    }

    pub(crate) fn substitute_gateway(&self, gateway: gateway::Node) -> NymTopology {
        let mut good_topology = self.good_v4_topology.clone();

        good_topology.set_gateways(vec![gateway]);
        good_topology
    }

    pub(crate) fn v4_topology(&self) -> &NymTopology {
        &self.good_v4_topology
    }

    pub(crate) fn v6_topology(&self) -> &NymTopology {
        &self.good_v6_topology
    }

    /// Given slices of bonded mixnodes and gateways, checks whether all 'good' nodes are present
    /// in the lists.
    ///
    /// # Arguments
    ///
    /// * `bonded_mixnodes`: slice of currently bonded mixnodes
    /// * `bonded_gateways`: slice of currently bonded gateways
    pub(crate) fn is_online(
        &self,
        bonded_mixnodes: &[MixNodeBond],
        bonded_gateways: &[GatewayBond],
    ) -> bool {
        // while technically this is not the most optimal way of checking all nodes as we have to
        // go through entire slice multiple times, we only do it every 30s before monitor startup
        // so it's not really that bad
        for layer_mixes in self.good_v4_topology.mixes().values() {
            for mix in layer_mixes {
                if !bonded_mixnodes.iter().any(|bonded| {
                    bonded.mix_node.identity_key == mix.identity_key.to_base58_string()
                }) {
                    return false;
                }
            }
        }

        for gateway in self.good_v4_topology.gateways() {
            if !bonded_gateways.iter().any(|bonded| {
                bonded.gateway.identity_key == gateway.identity_key.to_base58_string()
            }) {
                return false;
            }
        }

        true
    }
}
