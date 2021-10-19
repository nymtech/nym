// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::test_packet::{NodeType, TestPacket};
use crate::network_monitor::ROUTE_TESTING_TEST_NONCE;
use crypto::asymmetric::identity;
use mixnet_contract::{GatewayBond, MixNodeBond};
use std::fmt::{Debug, Formatter};
use topology::{gateway, mix, NymTopology};

#[derive(Clone)]
pub(crate) struct TestRoute {
    id: u64,
    system_version: String,
    nodes: NymTopology,
}

impl TestRoute {
    pub(crate) fn new(
        id: u64,
        system_version: &str,
        l1_mix: mix::Node,
        l2_mix: mix::Node,
        l3_mix: mix::Node,
        gateway: gateway::Node,
    ) -> Self {
        // When somebody gets to refactor this in the future and Rust 2021 is being used,
        // the call could be changed to a simple `.into_iter()`
        let layered_mixes = IntoIterator::into_iter([
            (1u8, vec![l1_mix]),
            (2u8, vec![l2_mix]),
            (3u8, vec![l3_mix]),
        ])
        .collect();

        TestRoute {
            id,
            system_version: system_version.to_string(),
            nodes: NymTopology::new(layered_mixes, vec![gateway]),
        }
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn gateway(&self) -> &gateway::Node {
        &self.nodes.gateways()[0]
    }

    pub(crate) fn layer_one_mix(&self) -> &mix::Node {
        &self.nodes.mixes().get(&1).unwrap()[0]
    }

    pub(crate) fn layer_two_mix(&self) -> &mix::Node {
        &self.nodes.mixes().get(&2).unwrap()[0]
    }

    pub(crate) fn layer_three_mix(&self) -> &mix::Node {
        &self.nodes.mixes().get(&3).unwrap()[0]
    }

    pub(crate) fn gateway_clients_address(&self) -> String {
        self.gateway().clients_address()
    }

    pub(crate) fn gateway_identity(&self) -> identity::PublicKey {
        self.gateway().identity_key
    }

    pub(crate) fn topology(&self) -> &NymTopology {
        &self.nodes
    }

    pub(crate) fn self_test_packet(&self) -> TestPacket {
        // it doesn't really matter which node is "chosen" as the packet has to always
        // go through the same sequence of hops.
        // let's just use layer 1 mixnode for this (this choice is completely arbitrary)
        let mix = &self.nodes.mixes()[&1][0];
        TestPacket::new(
            mix.identity_key,
            mix.owner.clone(),
            self.id,
            ROUTE_TESTING_TEST_NONCE,
            NodeType::Mixnode,
        )
    }

    pub(crate) fn system_version(&self) -> &str {
        &self.system_version
    }

    pub(crate) fn substitute_mix(&self, node: &mix::Node) -> NymTopology {
        let mut topology = self.nodes.clone();
        topology.set_mixes_in_layer(node.layer as u8, vec![node.clone()]);
        topology
    }

    pub(crate) fn substitute_gateway(&self, gateway: &gateway::Node) -> NymTopology {
        let mut topology = self.nodes.clone();
        topology.set_gateways(vec![gateway.clone()]);
        topology
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
        todo!()

        // // while technically this is not the most optimal way of checking all nodes as we have to
        // // go through entire slice multiple times, we only do it every 30s before monitor startup
        // // so it's not really that bad
        // for layer_mixes in self.good_v4_topology.mixes().values() {
        //     for mix in layer_mixes {
        //         if !bonded_mixnodes.iter().any(|bonded| {
        //             bonded.mix_node.identity_key == mix.identity_key.to_base58_string()
        //         }) {
        //             return false;
        //         }
        //     }
        // }
        //
        // for gateway in self.good_v4_topology.gateways() {
        //     if !bonded_gateways.iter().any(|bonded| {
        //         bonded.gateway.identity_key == gateway.identity_key.to_base58_string()
        //     }) {
        //         return false;
        //     }
        // }
        //
        // true
    }
}

impl Debug for TestRoute {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "[v{}] Route {}: [G] {} => [M1] {} => [M2] {} => [M3] {}",
            self.system_version,
            self.id,
            self.nodes.gateways()[0].identity().to_base58_string(),
            self.nodes.mixes_in_layer(1)[0]
                .identity_key
                .to_base58_string(),
            self.nodes.mixes_in_layer(2)[0]
                .identity_key
                .to_base58_string(),
            self.nodes.mixes_in_layer(3)[0]
                .identity_key
                .to_base58_string()
        )
    }
}
