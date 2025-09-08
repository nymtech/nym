// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::network_monitor::monitor::sender::GatewayPackets;
use crate::network_monitor::test_route::TestRoute;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_describe_cache::NodeDescriptionTopologyExt;
use crate::node_status_api::NodeStatusCache;
use crate::support::caching::cache::SharedCache;
use nym_api_requests::models::{NodeAnnotation, NymNodeDescription};
use nym_contracts_common::NaiveFloat;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_mixnet_contract_common::{LegacyMixLayer, NodeId};
use nym_node_tester_utils::node::{NodeType, TestableNode};
use nym_node_tester_utils::NodeTester;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::{PacketSize, PacketType};
use nym_topology::node::RoutingNode;
use rand::prelude::SliceRandom;
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, trace};

const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_AVERAGE_ACK_DELAY: Duration = Duration::from_millis(200);

pub(crate) struct PreparedPackets {
    /// All packets that are going to get sent during the test as well as the gateways through
    /// which they ought to be sent.
    pub(super) packets: Vec<GatewayPackets>,

    /// Vector containing list of public keys and owners of all nodes mixnodes being tested.
    pub(super) mixnodes_under_test: Vec<TestableNode>,

    /// Vector containing list of public keys and owners of all gateways being tested.
    pub(super) gateways_under_test: Vec<TestableNode>,
}

#[derive(Clone)]
pub(crate) struct PacketPreparer {
    contract_cache: MixnetContractCache,
    described_cache: SharedCache<DescribedNodes>,
    node_status_cache: NodeStatusCache,

    /// Number of test packets sent to each node
    per_node_test_packets: usize,

    ack_key: Arc<AckKey>,
    // TODO: security:
    // in the future we should really create unique set of keys every time otherwise
    // gateways might recognise our "test" keys and take special care to always forward those packets
    // even if otherwise they are malicious.
    self_public_identity: ed25519::PublicKey,
    self_public_encryption: x25519::PublicKey,
}

impl PacketPreparer {
    pub(crate) fn new(
        contract_cache: MixnetContractCache,
        described_cache: SharedCache<DescribedNodes>,
        node_status_cache: NodeStatusCache,
        per_node_test_packets: usize,
        ack_key: Arc<AckKey>,
        self_public_identity: ed25519::PublicKey,
        self_public_encryption: x25519::PublicKey,
    ) -> Self {
        PacketPreparer {
            contract_cache,
            described_cache,
            node_status_cache,
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
            false,
            DEFAULT_AVERAGE_PACKET_DELAY,
            DEFAULT_AVERAGE_ACK_DELAY,
            true,
            self.ack_key.clone(),
        )
    }

    // when we're testing mixnodes, the recipient is going to stay constant, so we can specify it ahead of time
    fn ephemeral_mix_tester(&self, test_route: &TestRoute) -> NodeTester<ThreadRng> {
        let self_address = self.create_packet_sender(&test_route.gateway());
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
        // wait for the caches to get initialised
        self.contract_cache.naive_wait_for_initial_values().await;
        self.described_cache.naive_wait_for_initial_values().await;

        // now wait for at least `minimum_full_routes` mixnodes per layer and `minimum_full_routes` gateway to be online
        info!("Waiting for minimal topology to be online");
        let initialisation_backoff = Duration::from_secs(30);
        loop {
            let nym_nodes = self.contract_cache.nym_nodes().await;

            #[allow(clippy::expect_used)]
            let described_nodes = self
                .described_cache
                .get()
                .await
                .expect("the self-describe cache should have been initialised!");

            let mut gateways_count = 0;
            let mut mixnodes_count = 0;

            for nym_node in nym_nodes {
                if let Some(described) = described_nodes.get_description(&nym_node.node_id()) {
                    if described.declared_role.mixnode {
                        mixnodes_count += 1;
                    } else if described.declared_role.entry {
                        gateways_count += 1;
                    }
                }
            }

            debug!(
                "we have {mixnodes_count} possible mixnodes and {gateways_count} possible gateways"
            );

            if gateways_count >= minimum_full_routes && mixnodes_count * 3 >= minimum_full_routes {
                break;
            }

            self.topology_wait_backoff(initialisation_backoff).await;
        }
    }

    fn random_legacy_layer<R: Rng>(&self, rng: &mut R) -> LegacyMixLayer {
        let layer_choices = [
            LegacyMixLayer::One,
            LegacyMixLayer::Two,
            LegacyMixLayer::Three,
        ];

        // SAFETY: the slice is not empty so the unwrap is fine
        #[allow(clippy::unwrap_used)]
        layer_choices.choose(rng).copied().unwrap()
    }

    fn to_legacy_layered_mixes<'a, R: Rng>(
        &self,
        rng: &mut R,
        current_rotation_id: u32,
        node_statuses: &HashMap<NodeId, NodeAnnotation>,
        mixing_nym_nodes: impl Iterator<Item = &'a NymNodeDescription> + 'a,
    ) -> HashMap<LegacyMixLayer, Vec<(RoutingNode, f64)>> {
        let mut layered_mixes = HashMap::new();

        for mixing_nym_node in mixing_nym_nodes {
            let Some(parsed_node) =
                self.nym_node_to_routing_node(current_rotation_id, mixing_nym_node)
            else {
                continue;
            };
            // if the node is not present, default to 0.5
            let weight = node_statuses
                .get(&mixing_nym_node.node_id)
                .map(|node| node.last_24h_performance.naive_to_f64())
                .unwrap_or(0.5);
            let layer = self.random_legacy_layer(rng);
            let layer_mixes = layered_mixes.entry(layer).or_insert_with(Vec::new);
            layer_mixes.push((parsed_node, weight))
        }

        layered_mixes
    }

    fn to_legacy_gateway_nodes<'a>(
        &self,
        current_rotation_id: u32,
        node_statuses: &HashMap<NodeId, NodeAnnotation>,
        gateway_capable_nym_nodes: impl Iterator<Item = &'a NymNodeDescription> + 'a,
    ) -> Vec<(RoutingNode, f64)> {
        let mut gateways = Vec::new();

        for gateway_capable_node in gateway_capable_nym_nodes {
            let Some(parsed_node) =
                self.nym_node_to_routing_node(current_rotation_id, gateway_capable_node)
            else {
                continue;
            };
            // if the node is not present, default to 0.5
            let weight = node_statuses
                .get(&gateway_capable_node.node_id)
                .map(|node| node.last_24h_performance.naive_to_f64())
                .unwrap_or(0.5);
            gateways.push((parsed_node, weight))
        }

        gateways
    }

    // chooses n random nodes from each layer (and gateway) such that they are not on the blacklist
    // if failed to get parsed => onto the blacklist they go
    // if generated fewer than n, blacklist will be updated by external function with correctly generated
    // routes so that they wouldn't be reused
    pub(crate) async fn prepare_test_routes(&self, n: usize) -> Option<Vec<TestRoute>> {
        let descriptions = self.described_cache.get().await.ok()?;
        let statuses = self.node_status_cache.node_annotations().await?;

        let mixing_nym_nodes = descriptions.mixing_nym_nodes();
        // last I checked `gatewaying` wasn't a word : )
        let gateway_capable_nym_nodes = descriptions.entry_capable_nym_nodes();

        // SAFETY: cache has already been initialised
        #[allow(clippy::unwrap_used)]
        let current_rotation_id = self.contract_cache.current_key_rotation_id().await.unwrap();

        let mut rng = thread_rng();

        // separate mixes into layers for easier selection alongside the selection weights
        let layered_mixes = self.to_legacy_layered_mixes(
            &mut rng,
            current_rotation_id,
            &statuses,
            mixing_nym_nodes,
        );
        let gateways =
            self.to_legacy_gateway_nodes(current_rotation_id, &statuses, gateway_capable_nym_nodes);

        // get all nodes from each layer...
        let l1 = layered_mixes.get(&LegacyMixLayer::One)?;
        let l2 = layered_mixes.get(&LegacyMixLayer::Two)?;
        let l3 = layered_mixes.get(&LegacyMixLayer::Three)?;

        // try to choose n nodes from each of them (+ gateways)...
        let rand_l1 = l1
            .choose_multiple_weighted(&mut rng, n, |item| item.1)
            .ok()?
            .map(|node| node.0.clone())
            .collect::<Vec<_>>();
        let rand_l2 = l2
            .choose_multiple_weighted(&mut rng, n, |item| item.1)
            .ok()?
            .map(|node| node.0.clone())
            .collect::<Vec<_>>();
        let rand_l3 = l3
            .choose_multiple_weighted(&mut rng, n, |item| item.1)
            .ok()?
            .map(|node| node.0.clone())
            .collect::<Vec<_>>();
        let rand_gateways = gateways
            .choose_multiple_weighted(&mut rng, n, |item| item.1)
            .ok()?
            .map(|node| node.0.clone())
            .collect::<Vec<_>>();

        // the unwrap on `min()` is fine as we know the iterator is not empty
        #[allow(clippy::unwrap_used)]
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
            let node_1 = rand_l1[i].clone();
            let node_2 = rand_l2[i].clone();
            let node_3 = rand_l3[i].clone();
            let gateway = rand_gateways[i].clone();

            routes.push(TestRoute::new(
                rng.gen(),
                current_rotation_id,
                node_1,
                node_2,
                node_3,
                gateway,
            ))
        }
        info!("The following routes will be used for testing: {routes:#?}");
        Some(routes)
    }

    fn create_packet_sender(&self, gateway: &RoutingNode) -> Recipient {
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
        let topology = route.testable_route_provider();

        let plaintexts = route.self_test_messages(num);

        // the unwrap here is fine as:
        // 1. the topology is definitely valid (otherwise we wouldn't be here)
        // 2. the recipient is specified (by calling **mix**_tester)
        // 3. the test message is not too long, i.e. when serialized it will fit in a single sphinx packet
        #[allow(clippy::unwrap_used)]
        let mix_packets = plaintexts
            .into_iter()
            .map(|p| tester.wrap_plaintext_data(p, &topology, None).unwrap())
            .map(MixPacket::from)
            .collect();

        GatewayPackets::new(
            route.gateway_clients_address(),
            route.gateway_identity(),
            mix_packets,
        )
    }

    fn nym_node_to_routing_node(
        &self,
        current_rotation_id: u32,
        description: &NymNodeDescription,
    ) -> Option<RoutingNode> {
        description.try_to_topology_node(current_rotation_id).ok()
    }

    pub(super) async fn prepare_test_packets(
        &mut self,
        test_nonce: u64,
        test_routes: &[TestRoute],
        // TODO: Maybe do this
        _packet_type: PacketType,
    ) -> PreparedPackets {
        // SAFETY: cache has already been initialised
        #[allow(clippy::unwrap_used)]
        let current_rotation_id = self.contract_cache.current_key_rotation_id().await.unwrap();

        #[allow(clippy::expect_used)]
        let descriptions = self
            .described_cache
            .get()
            .await
            .expect("the cache must have been initialised!");
        let mixing_nym_nodes = descriptions.mixing_nym_nodes();
        let gateway_capable_nym_nodes = descriptions.entry_capable_nym_nodes();

        let mut mixnodes_to_test_details = Vec::new();
        let mut gateways_to_test_details = Vec::new();
        let mut mixnodes_under_test = Vec::new();
        let mut gateways_under_test = Vec::new();

        // try to add nym-nodes into the fold
        for mix in mixing_nym_nodes {
            if let Some(parsed) = self.nym_node_to_routing_node(current_rotation_id, mix) {
                mixnodes_under_test.push(TestableNode::new_routing(&parsed, NodeType::Mixnode));
                mixnodes_to_test_details.push(parsed);
            }
        }

        // assign random layer to each node
        let mut rng = thread_rng();
        let mixnodes_to_test_details = mixnodes_to_test_details
            .into_iter()
            .map(|node| (self.random_legacy_layer(&mut rng), node))
            .collect::<Vec<_>>();

        for gateway in gateway_capable_nym_nodes {
            if let Some(parsed) = self.nym_node_to_routing_node(current_rotation_id, gateway) {
                gateways_under_test.push(TestableNode::new_routing(&parsed, NodeType::Gateway));
                gateways_to_test_details.push(parsed);
            }
        }

        let packets_to_create = (test_routes.len() * self.per_node_test_packets)
            * (mixnodes_under_test.len() + gateways_under_test.len());
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
            #[allow(clippy::unwrap_used)]
            let mixnode_test_packets = mix_tester
                .mixnodes_test_packets(
                    &mixnodes_to_test_details,
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
            for gateway in &gateways_to_test_details {
                let recipient = self.create_packet_sender(gateway);
                let gateway_identity = gateway.identity_key;
                let gateway_address = gateway.ws_entry_address(false);

                // the unwrap here is fine as:
                // 1. the topology is definitely valid (otherwise we wouldn't be here)
                // 2. the recipient is specified
                // 3. the test message is not too long, i.e. when serialized it will fit in a single sphinx packet
                #[allow(clippy::unwrap_used)]
                let gateway_test_packets = mix_tester
                    .legacy_gateway_test_packets(
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
            mixnodes_under_test,
            gateways_under_test,
        }
    }
}
