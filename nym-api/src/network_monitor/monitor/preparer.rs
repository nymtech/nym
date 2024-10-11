// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::monitor::sender::GatewayPackets;
use crate::network_monitor::test_route::TestRoute;
use crate::node_describe_cache::{DescribedNodes, NodeDescriptionTopologyExt};
use crate::nym_contract_cache::cache::{CachedRewardedSet, NymContractCache};
use crate::support::caching::cache::SharedCache;
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeBondWithLayer};
use nym_api_requests::models::NymNodeDescription;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_contract_common::{LegacyMixLayer, NodeId};
use nym_node_tester_utils::node::TestableNode;
use nym_node_tester_utils::NodeTester;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::{PacketSize, PacketType};
use nym_topology::gateway::GatewayConversionError;
use nym_topology::mix::MixnodeConversionError;
use nym_topology::{gateway, mix};
use rand::prelude::SliceRandom;
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, trace};

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
    contract_cache: NymContractCache,
    described_cache: SharedCache<DescribedNodes>,

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
        contract_cache: NymContractCache,
        described_cache: SharedCache<DescribedNodes>,
        per_node_test_packets: usize,
        ack_key: Arc<AckKey>,
        self_public_identity: identity::PublicKey,
        self_public_encryption: encryption::PublicKey,
    ) -> Self {
        PacketPreparer {
            contract_cache,
            described_cache,
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
        // wait for the caches to get initialised
        self.contract_cache.wait_for_initial_values().await;
        self.described_cache.naive_wait_for_initial_values().await;

        // now wait for at least `minimum_full_routes` mixnodes per layer and `minimum_full_routes` gateway to be online
        info!("Waiting for minimal topology to be online");
        let initialisation_backoff = Duration::from_secs(30);
        loop {
            let gateways = self.contract_cache.legacy_gateways_all().await;
            let mixnodes = self.contract_cache.legacy_mixnodes_all_basic().await;

            if gateways.len() < minimum_full_routes {
                self.topology_wait_backoff(initialisation_backoff).await;
                continue;
            }

            let mut layer1_count = 0;
            let mut layer2_count = 0;
            let mut layer3_count = 0;

            for mix in mixnodes {
                match mix.layer {
                    LegacyMixLayer::One => layer1_count += 1,
                    LegacyMixLayer::Two => layer2_count += 1,
                    LegacyMixLayer::Three => layer3_count += 1,
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

    async fn all_legacy_mixnodes_and_gateways(
        &self,
    ) -> (
        Vec<LegacyMixNodeBondWithLayer>,
        Vec<LegacyGatewayBondWithId>,
    ) {
        info!("Obtaining network topology...");

        let mixnodes = self.contract_cache.legacy_mixnodes_all_basic().await;
        let gateways = self.contract_cache.legacy_gateways_all().await;

        (mixnodes, gateways)
    }

    async fn filtered_legacy_mixnodes_and_gateways(
        &self,
    ) -> (
        Vec<LegacyMixNodeBondWithLayer>,
        Vec<LegacyGatewayBondWithId>,
    ) {
        info!("Obtaining network topology...");

        let mixnodes = self.contract_cache.legacy_mixnodes_filtered_basic().await;
        let gateways = self.contract_cache.legacy_gateways_filtered().await;

        (mixnodes, gateways)
    }

    pub(crate) fn try_parse_mix_bond(
        &self,
        bond: &LegacyMixNodeBondWithLayer,
    ) -> Result<mix::LegacyNode, String> {
        fn parse_bond(
            bond: &LegacyMixNodeBondWithLayer,
        ) -> Result<mix::LegacyNode, MixnodeConversionError> {
            let host = mix::LegacyNode::parse_host(&bond.mix_node.host)?;

            // try to completely resolve the host in the mix situation to avoid doing it every
            // single time we want to construct a path
            let mix_host = mix::LegacyNode::extract_mix_host(&host, bond.mix_node.mix_port)?;

            Ok(mix::LegacyNode {
                mix_id: bond.mix_id,
                host,
                mix_host,
                identity_key: identity::PublicKey::from_base58_string(&bond.mix_node.identity_key)?,
                sphinx_key: encryption::PublicKey::from_base58_string(&bond.mix_node.sphinx_key)?,
                layer: bond.layer,
                version: bond.mix_node.version.as_str().into(),
            })
        }

        let identity = bond.mix_node.identity_key.clone();
        parse_bond(bond).map_err(|_| identity)
    }

    pub(crate) fn try_parse_gateway_bond(
        &self,
        gateway: &LegacyGatewayBondWithId,
    ) -> Result<gateway::LegacyNode, String> {
        fn parse_bond(
            bond: &LegacyGatewayBondWithId,
        ) -> Result<gateway::LegacyNode, GatewayConversionError> {
            let host = gateway::LegacyNode::parse_host(&bond.gateway.host)?;

            // try to completely resolve the host in the mix situation to avoid doing it every
            // single time we want to construct a path
            let mix_host = gateway::LegacyNode::extract_mix_host(&host, bond.gateway.mix_port)?;

            Ok(gateway::LegacyNode {
                node_id: bond.node_id,
                host,
                mix_host,
                clients_ws_port: bond.gateway.clients_port,
                clients_wss_port: None,
                identity_key: identity::PublicKey::from_base58_string(&bond.gateway.identity_key)?,
                sphinx_key: encryption::PublicKey::from_base58_string(&bond.gateway.sphinx_key)?,
                version: bond.gateway.version.as_str().into(),
            })
        }

        let identity = gateway.gateway.identity_key.clone();
        parse_bond(gateway).map_err(|_| identity)
    }

    fn layered_mixes<'a, R: Rng>(
        &self,
        rng: &mut R,
        blacklist: &mut HashSet<NodeId>,
        rewarded_set: &CachedRewardedSet,
        legacy_mixnodes: Vec<LegacyMixNodeBondWithLayer>,
        mixing_nym_nodes: impl Iterator<Item = &'a NymNodeDescription> + 'a,
    ) -> HashMap<LegacyMixLayer, Vec<mix::LegacyNode>> {
        let mut layered_mixes = HashMap::new();
        for mix in legacy_mixnodes {
            let layer = mix.layer;
            let layer_mixes = layered_mixes.entry(layer).or_insert_with(Vec::new);
            let Ok(parsed_node) = self.try_parse_mix_bond(&mix) else {
                blacklist.insert(mix.mix_id);
                continue;
            };
            layer_mixes.push(parsed_node)
        }

        for mixing_nym_node in mixing_nym_nodes {
            let Some(parsed_node) = self.nym_node_to_legacy_mix(rng, rewarded_set, mixing_nym_node)
            else {
                continue;
            };
            let layer = parsed_node.layer;
            let layer_mixes = layered_mixes.entry(layer).or_insert_with(Vec::new);
            layer_mixes.push(parsed_node)
        }

        layered_mixes
    }

    fn all_gateways<'a>(
        &self,
        blacklist: &mut HashSet<NodeId>,
        legacy_gateways: Vec<LegacyGatewayBondWithId>,
        gateway_capable_nym_nodes: impl Iterator<Item = &'a NymNodeDescription> + 'a,
    ) -> Vec<gateway::LegacyNode> {
        let mut gateways = Vec::new();
        for gateway in legacy_gateways {
            let Ok(parsed_node) = self.try_parse_gateway_bond(&gateway) else {
                blacklist.insert(gateway.node_id);
                continue;
            };
            gateways.push(parsed_node)
        }

        for gateway_capable_node in gateway_capable_nym_nodes {
            let Some(parsed_node) = self.nym_node_to_legacy_gateway(gateway_capable_node) else {
                continue;
            };
            gateways.push(parsed_node)
        }

        gateways
    }

    // chooses n random nodes from each layer (and gateway) such that they are not on the blacklist
    // if failed to get parsed => onto the blacklist they go
    // if generated fewer than n, blacklist will be updated by external function with correctly generated
    // routes so that they wouldn't be reused
    pub(crate) async fn prepare_test_routes(
        &self,
        n: usize,
        blacklist: &mut HashSet<NodeId>,
    ) -> Option<Vec<TestRoute>> {
        let (legacy_mixnodes, legacy_gateways) = self.filtered_legacy_mixnodes_and_gateways().await;
        let rewarded_set = self.contract_cache.rewarded_set().await?;

        let descriptions = self.described_cache.get().await.ok()?;

        let mixing_nym_nodes = descriptions.mixing_nym_nodes();
        // last I checked `gatewaying` wasn't a word : )
        let gateway_capable_nym_nodes = descriptions.entry_capable_nym_nodes();

        let mut rng = thread_rng();

        // separate mixes into layers for easier selection
        let layered_mixes = self.layered_mixes(
            &mut rng,
            blacklist,
            &rewarded_set,
            legacy_mixnodes,
            mixing_nym_nodes,
        );
        let gateways = self.all_gateways(blacklist, legacy_gateways, gateway_capable_nym_nodes);

        // get all nodes from each layer...
        let l1 = layered_mixes.get(&LegacyMixLayer::One)?;
        let l2 = layered_mixes.get(&LegacyMixLayer::Two)?;
        let l3 = layered_mixes.get(&LegacyMixLayer::Three)?;

        // try to choose n nodes from each of them (+ gateways)...
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
            let node_1 = rand_l1[i].clone();
            let node_2 = rand_l2[i].clone();
            let node_3 = rand_l3[i].clone();
            let gateway = rand_gateways[i].clone();

            routes.push(TestRoute::new(rng.gen(), node_1, node_2, node_3, gateway))
        }
        info!("The following routes will be used for testing: {routes:#?}");
        Some(routes)
    }

    fn create_packet_sender(&self, gateway: &gateway::LegacyNode) -> Recipient {
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
        nodes: Vec<LegacyMixNodeBondWithLayer>,
    ) -> (Vec<mix::LegacyNode>, Vec<InvalidNode>) {
        let mut parsed_nodes = Vec::new();
        let mut invalid_nodes = Vec::new();
        for mixnode in nodes {
            if let Ok(parsed_node) = self.try_parse_mix_bond(&mixnode) {
                parsed_nodes.push(parsed_node)
            } else {
                invalid_nodes.push(InvalidNode::Malformed {
                    node: TestableNode::new_mixnode(mixnode.identity().to_owned(), mixnode.mix_id),
                });
            }
        }
        (parsed_nodes, invalid_nodes)
    }

    fn filter_outdated_and_malformed_gateways(
        &self,
        nodes: Vec<LegacyGatewayBondWithId>,
    ) -> (Vec<(gateway::LegacyNode, NodeId)>, Vec<InvalidNode>) {
        let mut parsed_nodes = Vec::new();
        let mut invalid_nodes = Vec::new();
        for gateway in nodes {
            if let Ok(parsed_node) = self.try_parse_gateway_bond(&gateway) {
                parsed_nodes.push((parsed_node, gateway.node_id))
            } else {
                invalid_nodes.push(InvalidNode::Malformed {
                    node: TestableNode::new_gateway(
                        gateway.bond.identity().to_owned(),
                        gateway.node_id,
                    ),
                });
            }
        }
        (parsed_nodes, invalid_nodes)
    }

    fn nym_node_to_legacy_mix<R: Rng>(
        &self,
        rng: &mut R,
        rewarded_set: &CachedRewardedSet,
        mixing_nym_node: &NymNodeDescription,
    ) -> Option<mix::LegacyNode> {
        let maybe_explicit_layer = rewarded_set
            .try_get_mix_layer(&mixing_nym_node.node_id)
            .and_then(|layer| LegacyMixLayer::try_from(layer).ok());

        let layer = match maybe_explicit_layer {
            Some(layer) => layer,
            None => {
                let layer_choices = [
                    LegacyMixLayer::One,
                    LegacyMixLayer::Two,
                    LegacyMixLayer::Three,
                ];

                // if nym-node doesn't have a layer assigned, since it's either standby or inactive,
                // we have to choose one randomly for the testing purposes
                // SAFETY: the slice is not empty so the unwrap is fine
                #[allow(clippy::unwrap_used)]
                layer_choices.choose(rng).copied().unwrap()
            }
        };

        mixing_nym_node.try_to_topology_mix_node(layer).ok()
    }

    fn nym_node_to_legacy_gateway(
        &self,
        gateway_capable_node: &NymNodeDescription,
    ) -> Option<gateway::LegacyNode> {
        gateway_capable_node.try_to_topology_gateway().ok()
    }

    pub(super) async fn prepare_test_packets(
        &mut self,
        test_nonce: u64,
        test_routes: &[TestRoute],
        // TODO: Maybe do this
        _packet_type: PacketType,
    ) -> PreparedPackets {
        let (mixnodes, gateways) = self.all_legacy_mixnodes_and_gateways().await;
        let rewarded_set = self.contract_cache.rewarded_set().await;

        let descriptions = self
            .described_cache
            .get()
            .await
            .expect("the cache must have been initialised!");
        let mixing_nym_nodes = descriptions.mixing_nym_nodes();
        let gateway_capable_nym_nodes = descriptions.entry_capable_nym_nodes();

        let (mixnodes, invalid_mixnodes) = self.filter_outdated_and_malformed_mixnodes(mixnodes);
        let (gateways, invalid_gateways) = self.filter_outdated_and_malformed_gateways(gateways);

        let mut tested_mixnodes = mixnodes.iter().map(|node| node.into()).collect::<Vec<_>>();
        let mut tested_gateways = gateways.iter().map(|node| node.into()).collect::<Vec<_>>();

        // try to add nym-nodes into the fold
        if let Some(rewarded_set) = rewarded_set {
            let mut rng = thread_rng();
            for mix in mixing_nym_nodes {
                if let Some(parsed) = self.nym_node_to_legacy_mix(&mut rng, &rewarded_set, mix) {
                    tested_mixnodes.push(TestableNode::from(&parsed));
                }
            }
        }

        for gateway in gateway_capable_nym_nodes {
            if let Some(parsed) = self.nym_node_to_legacy_gateway(gateway) {
                tested_gateways.push((&parsed, gateway.node_id).into())
            }
        }

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
            #[allow(clippy::unwrap_used)]
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
            for (gateway, node_id) in &gateways {
                let recipient = self.create_packet_sender(gateway);
                let gateway_identity = gateway.identity_key;
                let gateway_address = gateway.clients_address();

                // the unwrap here is fine as:
                // 1. the topology is definitely valid (otherwise we wouldn't be here)
                // 2. the recipient is specified
                // 3. the test message is not too long, i.e. when serialized it will fit in a single sphinx packet
                #[allow(clippy::unwrap_used)]
                let gateway_test_packets = mix_tester
                    .legacy_gateway_test_packets(
                        gateway,
                        *node_id,
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
