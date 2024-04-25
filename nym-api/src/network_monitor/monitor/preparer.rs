// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::monitor::sender::GatewayPackets;
use crate::network_monitor::test_route::TestRoute;
use crate::nym_contract_cache::cache::NymContractCache;
use log::info;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_contract_common::{GatewayBond, Layer, MixNodeBond};
use nym_node_tester_utils::node::TestableNode;
use nym_node_tester_utils::NodeTester;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::{PacketSize, PacketType};
use nym_topology::{gateway, mix};
use rand_07::{rngs::ThreadRng, seq::SliceRandom, thread_rng, Rng};
use std::collections::{HashMap, HashSet};

use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_AVERAGE_ACK_DELAY: Duration = Duration::from_millis(200);

#[derive(Clone)]
pub(crate) enum InvalidNode {
    Malformed { node: TestableNode },
}

impl Display for InvalidNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InvalidNode::Malformed { node } => {
                write!(f, "{node} is malformed")
            }
        }
    }
}

impl From<InvalidNode> for TestableNode {
    fn from(value: InvalidNode) -> Self {
        match value {
            InvalidNode::Malformed { node } => node,
        }
    }
}

pub(crate) struct PreparedPackets {
    /// All packets that are going to get sent during the test as well as the gateways through
    /// which they ought to be sent.
    pub(super) packets: Vec<GatewayPackets>,

    /// Vector containing list of public keys and owners of all nodes mixnodes being tested.
    pub(super) tested_mixnodes: Vec<TestableNode>,

    /// Vector containing list of public keys and owners of all gateways being tested.
    pub(super) tested_gateways: Vec<TestableNode>,

    /// All mixnodes that failed to get parsed correctly or were not version compatible.
    /// They will be marked to the validator as being down for the test.
    pub(super) invalid_mixnodes: Vec<InvalidNode>,

    /// All gateways that failed to get parsed correctly or were not version compatible.
    /// They will be marked to the validator as being down for the test.
    pub(super) invalid_gateways: Vec<InvalidNode>,
}

#[derive(Clone)]
pub(crate) struct PacketPreparer {
    validator_cache: NymContractCache,

    /// Number of test packets sent to each node
    per_node_test_packets: usize,

    ack_key: Arc<AckKey>,
    // TODO: security:
    // in the future we should really create unique set of keys every time otherwise
    // gateways might recognise our "test" keys and take special care to always forward those packets
    // even if otherwise they are malicious.
    self_public_identity: identity::PublicKey,
    self_public_encryption: encryption::PublicKey,
}

impl PacketPreparer {
    pub(crate) fn new(
        validator_cache: NymContractCache,
        per_node_test_packets: usize,
        ack_key: Arc<AckKey>,
        self_public_identity: identity::PublicKey,
        self_public_encryption: encryption::PublicKey,
    ) -> Self {
        PacketPreparer {
            validator_cache,
            per_node_test_packets,
            ack_key,
            self_public_identity,
            self_public_encryption,
        }
    }

    fn ephemeral_tester(
        &self,
        test_route: &TestRoute,
        self_address: Option<Recipient>,
    ) -> NodeTester<ThreadRng> {
        let rng = thread_rng();
        NodeTester::new(
            rng,
            // the topology here contains 3 mixnodes and 1 gateway so its cheap to clone it
            test_route.topology().clone(),
            self_address,
            PacketSize::RegularPacket,
            DEFAULT_AVERAGE_PACKET_DELAY,
            DEFAULT_AVERAGE_ACK_DELAY,
            self.ack_key.clone(),
        )
    }

    // when we're testing mixnodes, the recipient is going to stay constant, so we can specify it ahead of time
    fn ephemeral_mix_tester(&self, test_route: &TestRoute) -> NodeTester<ThreadRng> {
        let self_address = self.create_packet_sender(test_route.gateway());
        self.ephemeral_tester(test_route, Some(self_address))
    }

    #[allow(dead_code)]
    fn ephemeral_gateway_tester(&self, test_route: &TestRoute) -> NodeTester<ThreadRng> {
        self.ephemeral_tester(test_route, None)
    }

    async fn topology_wait_backoff(&self, initialisation_backoff: Duration) {
        info!(
            "Minimal topology is still not online. Going to check again in {:?}",
            initialisation_backoff
        );
        tokio::time::sleep(initialisation_backoff).await;
    }

    pub(crate) async fn wait_for_validator_cache_initial_values(&self, minimum_full_routes: usize) {
        // wait for the cache to get initialised
        self.validator_cache.wait_for_initial_values().await;

        // now wait for at least `minimum_full_routes` mixnodes per layer and `minimum_full_routes` gateway to be online
        info!("Waiting for minimal topology to be online");
        let initialisation_backoff = Duration::from_secs(30);
        loop {
            let gateways = self.validator_cache.gateways_all().await;
            let mixnodes = self.validator_cache.mixnodes_all_basic().await;

            if gateways.len() < minimum_full_routes {
                self.topology_wait_backoff(initialisation_backoff).await;
                continue;
            }

            let mut layer1_count = 0;
            let mut layer2_count = 0;
            let mut layer3_count = 0;

            for mix in mixnodes {
                match mix.layer {
                    Layer::One => layer1_count += 1,
                    Layer::Two => layer2_count += 1,
                    Layer::Three => layer3_count += 1,
                }
            }

            if layer1_count >= minimum_full_routes
                && layer2_count >= minimum_full_routes
                && layer3_count >= minimum_full_routes
            {
                break;
            }

            self.topology_wait_backoff(initialisation_backoff).await;
        }
    }

    async fn all_mixnodes_and_gateways(&self) -> (Vec<MixNodeBond>, Vec<GatewayBond>) {
        info!("Obtaining network topology...");

        let mixnodes = self.validator_cache.mixnodes_all_basic().await;
        let gateways = self.validator_cache.gateways_all().await;

        (mixnodes, gateways)
    }

    async fn filtered_mixnodes_and_gateways(&self) -> (Vec<MixNodeBond>, Vec<GatewayBond>) {
        info!("Obtaining network topology...");

        let mixnodes = self.validator_cache.mixnodes_filtered_basic().await;
        let gateways = self.validator_cache.gateways_filtered().await;

        (mixnodes, gateways)
    }

    pub(crate) fn try_parse_mix_bond(&self, mix: &MixNodeBond) -> Result<mix::Node, String> {
        let identity = mix.mix_node.identity_key.clone();
        mix.try_into().map_err(|_| identity)
    }

    pub(crate) fn try_parse_gateway_bond(
        &self,
        gateway: &GatewayBond,
    ) -> Result<gateway::Node, String> {
        let identity = gateway.gateway.identity_key.clone();
        gateway.try_into().map_err(|_| identity)
    }

    // gets rewarded nodes
    // chooses n random nodes from each layer (and gateway) such that they are not on the blacklist
    // if failed to parsed => onto the blacklist they go
    // if generated fewer than n, blacklist will be updated by external function with correctly generated
    // routes so that they wouldn't be reused
    pub(crate) async fn prepare_test_routes(
        &self,
        n: usize,
        blacklist: &mut HashSet<String>,
    ) -> Option<Vec<TestRoute>> {
        let (mixnodes, gateways) = self.filtered_mixnodes_and_gateways().await;
        // separate mixes into layers for easier selection
        let mut layered_mixes = HashMap::new();
        for mix in mixnodes {
            let layer = mix.layer;
            let mixes = layered_mixes.entry(layer).or_insert_with(Vec::new);
            mixes.push(mix)
        }

        // get all nodes from each layer...
        let l1 = layered_mixes.get(&Layer::One)?;
        let l2 = layered_mixes.get(&Layer::Two)?;
        let l3 = layered_mixes.get(&Layer::Three)?;

        // try to choose n nodes from each of them (+ gateways)...
        let mut rng = thread_rng();

        let rand_l1 = l1.choose_multiple(&mut rng, n).collect::<Vec<_>>();
        let rand_l2 = l2.choose_multiple(&mut rng, n).collect::<Vec<_>>();
        let rand_l3 = l3.choose_multiple(&mut rng, n).collect::<Vec<_>>();
        let rand_gateways = gateways.choose_multiple(&mut rng, n).collect::<Vec<_>>();

        // the unwrap on `min()` is fine as we know the iterator is not empty
        let most_available = *[
            rand_l1.len(),
            rand_l2.len(),
            rand_l3.len(),
            rand_gateways.len(),
        ]
        .iter()
        .min()
        .unwrap();

        if most_available == 0 {
            error!("Cannot construct test routes. No nodes or gateways available");
            return None;
        }

        trace!("Generating test routes...");
        let mut routes = Vec::new();
        for i in 0..most_available {
            let Ok(node_1) = self.try_parse_mix_bond(rand_l1[i]) else {
                blacklist.insert(rand_l1[i].identity().to_owned());
                continue;
            };

            let Ok(node_2) = self.try_parse_mix_bond(rand_l2[i]) else {
                blacklist.insert(rand_l2[i].identity().to_owned());
                continue;
            };

            let Ok(node_3) = self.try_parse_mix_bond(rand_l3[i]) else {
                blacklist.insert(rand_l3[i].identity().to_owned());
                continue;
            };

            let Ok(gateway) = self.try_parse_gateway_bond(rand_gateways[i]) else {
                blacklist.insert(rand_gateways[i].identity().to_owned());
                continue;
            };

            routes.push(TestRoute::new(rng.gen(), node_1, node_2, node_3, gateway))
        }
        info!(
            "The following routes will be used for testing: {:#?}",
            routes
        );
        Some(routes)
    }

    fn create_packet_sender(&self, gateway: &gateway::Node) -> Recipient {
        Recipient::new(
            self.self_public_identity,
            self.self_public_encryption,
            gateway.identity_key,
        )
    }

    pub(crate) fn prepare_test_route_viability_packets(
        &mut self,
        route: &TestRoute,
        num: usize,
        // TODO: Maybe do this
        _packet_type: PacketType,
    ) -> GatewayPackets {
        let mut tester = self.ephemeral_mix_tester(route);
        let topology = route.topology();
        let plaintexts = route.self_test_messages(num);

        // the unwrap here is fine as:
        // 1. the topology is definitely valid (otherwise we wouldn't be here)
        // 2. the recipient is specified (by calling **mix**_tester)
        // 3. the test message is not too long, i.e. when serialized it will fit in a single sphinx packet
        let mix_packets = plaintexts
            .into_iter()
            .map(|p| tester.wrap_plaintext_data(p, topology, None).unwrap())
            .map(MixPacket::from)
            .collect();

        GatewayPackets::new(
            route.gateway_clients_address(),
            route.gateway_identity(),
            mix_packets,
        )
    }

    fn filter_outdated_and_malformed_mixnodes(
        &self,
        nodes: Vec<MixNodeBond>,
    ) -> (Vec<mix::Node>, Vec<InvalidNode>) {
        let mut parsed_nodes = Vec::new();
        let mut invalid_nodes = Vec::new();
        for mixnode in nodes {
            if let Ok(parsed_node) = (&mixnode).try_into() {
                parsed_nodes.push(parsed_node)
            } else {
                invalid_nodes.push(InvalidNode::Malformed {
                    node: TestableNode::new_mixnode(
                        mixnode.identity().to_owned(),
                        mixnode.owner.clone().into_string(),
                        mixnode.mix_id,
                    ),
                });
            }
        }
        (parsed_nodes, invalid_nodes)
    }

    fn filter_outdated_and_malformed_gateways(
        &self,
        nodes: Vec<GatewayBond>,
    ) -> (Vec<gateway::Node>, Vec<InvalidNode>) {
        let mut parsed_nodes = Vec::new();
        let mut invalid_nodes = Vec::new();
        for gateway in nodes {
            if let Ok(parsed_node) = (&gateway).try_into() {
                parsed_nodes.push(parsed_node)
            } else {
                invalid_nodes.push(InvalidNode::Malformed {
                    node: TestableNode::new_gateway(
                        gateway.identity().to_owned(),
                        gateway.owner.clone().into_string(),
                    ),
                });
            }
        }
        (parsed_nodes, invalid_nodes)
    }

    pub(super) async fn prepare_test_packets(
        &mut self,
        test_nonce: u64,
        test_routes: &[TestRoute],
        // TODO: Maybe do this
        _packet_type: PacketType,
    ) -> PreparedPackets {
        // only test mixnodes that are rewarded, i.e. that will be rewarded in this interval.
        // (remember that "idle" nodes are still part of that set)
        // we don't care about other nodes, i.e. nodes that are bonded but will not get
        // any reward during the current rewarding interval
        let (mixnodes, gateways) = self.all_mixnodes_and_gateways().await;

        let (mixnodes, invalid_mixnodes) = self.filter_outdated_and_malformed_mixnodes(mixnodes);
        let (gateways, invalid_gateways) = self.filter_outdated_and_malformed_gateways(gateways);

        let tested_mixnodes = mixnodes.iter().map(|node| node.into()).collect::<Vec<_>>();
        let tested_gateways = gateways.iter().map(|node| node.into()).collect::<Vec<_>>();

        let packets_to_create = (test_routes.len() * self.per_node_test_packets)
            * (tested_mixnodes.len() + tested_gateways.len());
        info!("Need to create {} mix packets", packets_to_create);

        let mut all_gateway_packets = HashMap::new();

        // for each test route...
        for test_route in test_routes {
            let route_ext = test_route.test_message_ext(test_nonce);
            let gateway_address = test_route.gateway_clients_address();
            let gateway_identity = test_route.gateway_identity();

            let mut mix_tester = self.ephemeral_mix_tester(test_route);

            // generate test packets for mixnodes
            //
            // the unwrap here is fine as:
            // 1. the topology is definitely valid (otherwise we wouldn't be here)
            // 2. the recipient is specified (by calling **mix**_tester)
            // 3. the test message is not too long, i.e. when serialized it will fit in a single sphinx packet
            let mixnode_test_packets = mix_tester
                .mixnodes_test_packets(
                    &mixnodes,
                    route_ext,
                    self.per_node_test_packets as u32,
                    None,
                )
                .unwrap();
            let mix_packets = mixnode_test_packets.into_iter().map(Into::into).collect();

            let gateway_packets = all_gateway_packets
                .entry(gateway_identity.to_bytes())
                .or_insert_with(|| GatewayPackets::empty(gateway_address, gateway_identity));
            gateway_packets.push_packets(mix_packets);

            // and generate test packets for gateways (note the variable recipient)
            for gateway in &gateways {
                let recipient = self.create_packet_sender(gateway);
                let gateway_identity = gateway.identity_key;
                let gateway_address = gateway.clients_address();

                // the unwrap here is fine as:
                // 1. the topology is definitely valid (otherwise we wouldn't be here)
                // 2. the recipient is specified
                // 3. the test message is not too long, i.e. when serialized it will fit in a single sphinx packet
                let gateway_test_packets = mix_tester
                    .gateway_test_packets(
                        gateway,
                        route_ext,
                        self.per_node_test_packets as u32,
                        Some(recipient),
                    )
                    .unwrap();
                let gateway_mix_packets =
                    gateway_test_packets.into_iter().map(Into::into).collect();

                // and push it into existing struct (if it's a "core" gateway being tested against another route)
                // or create a new one
                let gateway_packets = all_gateway_packets
                    .entry(gateway_identity.to_bytes())
                    .or_insert_with(|| GatewayPackets::empty(gateway_address, gateway_identity));
                gateway_packets.push_packets(gateway_mix_packets);
            }
        }

        // convert our hashmap back into a vec
        let packets = all_gateway_packets.into_values().collect();

        PreparedPackets {
            packets,
            tested_mixnodes,
            tested_gateways,
            invalid_mixnodes,
            invalid_gateways,
        }
    }
}
