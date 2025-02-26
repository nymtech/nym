// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::test_packet::NymApiTestMessageExt;
use crate::network_monitor::ROUTE_TESTING_TEST_NONCE;
use nym_crypto::asymmetric::identity;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::{EpochId, EpochRewardedSet, RewardedSet};
use nym_topology::node::RoutingNode;
use nym_topology::{NymRouteProvider, NymTopology};
use std::fmt::{Debug, Formatter};

#[derive(Clone)]
pub(crate) struct TestRoute {
    id: u64,
    nodes: NymTopology,
}

impl TestRoute {
    pub(crate) fn new(
        id: u64,
        l1_mix: RoutingNode,
        l2_mix: RoutingNode,
        l3_mix: RoutingNode,
        gateway: RoutingNode,
    ) -> Self {
        let fake_rewarded_set = EpochRewardedSet {
            epoch_id: EpochId::MAX,
            assignment: RewardedSet {
                entry_gateways: vec![gateway.node_id],
                exit_gateways: vec![],
                layer1: vec![l1_mix.node_id],
                layer2: vec![l2_mix.node_id],
                layer3: vec![l3_mix.node_id],
                standby: vec![],
            },
        };

        let nodes = vec![l1_mix, l2_mix, l3_mix, gateway];

        TestRoute {
            id,
            nodes: NymTopology::new(fake_rewarded_set, nodes),
        }
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn gateway(&self) -> RoutingNode {
        // SAFETY: we inserted entry gateway at construction
        #[allow(clippy::unwrap_used)]
        self.nodes
            .nodes_with_role(Role::EntryGateway)
            .next()
            .unwrap()
            .clone()
    }

    pub(crate) fn layer_one_mix(&self) -> &RoutingNode {
        // SAFETY: we inserted layer1 node at construction
        #[allow(clippy::unwrap_used)]
        self.nodes.nodes_with_role(Role::Layer1).next().unwrap()
    }

    pub(crate) fn layer_two_mix(&self) -> &RoutingNode {
        // SAFETY: we inserted layer2 node at construction
        #[allow(clippy::unwrap_used)]
        self.nodes.nodes_with_role(Role::Layer2).next().unwrap()
    }

    pub(crate) fn layer_three_mix(&self) -> &RoutingNode {
        // SAFETY: we inserted layer3 node at construction
        #[allow(clippy::unwrap_used)]
        self.nodes.nodes_with_role(Role::Layer3).next().unwrap()
    }

    pub(crate) fn gateway_clients_address(&self) -> Option<String> {
        self.gateway().ws_entry_address(false)
    }

    pub(crate) fn gateway_identity(&self) -> identity::PublicKey {
        self.gateway().identity_key
    }

    pub(crate) fn topology(&self) -> &NymTopology {
        &self.nodes
    }

    pub(crate) fn testable_route_provider(&self) -> NymRouteProvider {
        self.nodes.clone().into()
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
