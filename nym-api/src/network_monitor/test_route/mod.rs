// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::test_packet::NymApiTestMessageExt;
use crate::network_monitor::ROUTE_TESTING_TEST_NONCE;
use nym_crypto::asymmetric::identity;
use nym_topology::{gateway, mix, NymTopology};
use std::fmt::{Debug, Formatter};

#[derive(Clone)]
pub(crate) struct TestRoute {
    id: u64,
    nodes: NymTopology,
}

impl TestRoute {
    pub(crate) fn new(
        id: u64,
        l1_mix: mix::Node,
        l2_mix: mix::Node,
        l3_mix: mix::Node,
        gateway: gateway::Node,
    ) -> Self {
        let layered_mixes = [
            (1u8, vec![l1_mix]),
            (2u8, vec![l2_mix]),
            (3u8, vec![l3_mix]),
        ]
        .into_iter()
        .collect();

        TestRoute {
            id,
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

    pub(crate) fn test_message_ext(&self, test_nonce: u64) -> NymApiTestMessageExt {
        NymApiTestMessageExt::new(self.id, test_nonce)
    }

    pub(crate) fn self_test_messages(&self, count: usize) -> Vec<Vec<u8>> {
        // it doesn't really matter which node is "chosen" as the packet has to always
        // go through the same sequence of hops.
        // let's just use layer 1 mixnode for this (this choice is completely arbitrary)
        let mix = self.layer_one_mix();

        // the unwrap here is fine as the failure can only occur due to serialization and we're not
        // using any custom implementations
        NymApiTestMessageExt::new(self.id, ROUTE_TESTING_TEST_NONCE)
            .mix_plaintexts(mix, count as u32)
            .unwrap()
    }
}

impl Debug for TestRoute {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Route {}: [G] {} => [M1] {} => [M2] {} => [M3] {}",
            self.id,
            self.gateway().identity().to_base58_string(),
            self.layer_one_mix().identity_key.to_base58_string(),
            self.layer_two_mix().identity_key.to_base58_string(),
            self.layer_three_mix().identity_key.to_base58_string()
        )
    }
}
