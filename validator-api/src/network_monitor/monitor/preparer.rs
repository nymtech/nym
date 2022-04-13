// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_cache::ValidatorCache;
use crate::network_monitor::chunker::Chunker;
use crate::network_monitor::monitor::sender::GatewayPackets;
use crate::network_monitor::test_packet::{NodeType, TestPacket};
use crate::network_monitor::test_route::TestRoute;
use crypto::asymmetric::{encryption, identity};
use log::info;
use mixnet_contract_common::{Addr, GatewayBond, Layer, MixNodeBond};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::forwarding::packet::MixPacket;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;
use topology::{gateway, mix, NymTopology};

// declared type aliases for easier code reasoning
type Version = String;
type Id = String;
type Owner = Addr;

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) enum InvalidNode {
    Outdated(Id, Owner, Version),
    Malformed(Id, Owner),
}

impl Display for InvalidNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InvalidNode::Outdated(id, owner, version) => {
                write!(
                    f,
                    "Node {} (v{}) owned by {} is outdated",
                    id, version, owner
                )
            }
            InvalidNode::Malformed(id, owner) => {
                write!(f, "Node {} owned by {} is malformed", id, owner)
            }
        }
    }
}

impl InvalidNode {
    pub(crate) fn identity(&self) -> String {
        match self {
            InvalidNode::Outdated(id, _, _) => id.clone(),
            InvalidNode::Malformed(id, _) => id.clone(),
        }
    }

    pub(crate) fn owner(&self) -> String {
        match self {
            InvalidNode::Outdated(_, owner, _) => owner.into(),
            InvalidNode::Malformed(_, owner) => owner.into(),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Hash, Clone)]
pub(crate) struct TestedNode {
    pub(crate) identity: String,
    pub(crate) owner: String,
    pub(crate) node_type: NodeType,
}

impl<'a> From<&'a mix::Node> for TestedNode {
    fn from(node: &'a mix::Node) -> Self {
        TestedNode {
            identity: node.identity_key.to_base58_string(),
            owner: node.owner.clone(),
            node_type: NodeType::Mixnode,
        }
    }
}

impl<'a> From<&'a gateway::Node> for TestedNode {
    fn from(node: &'a gateway::Node) -> Self {
        TestedNode {
            identity: node.identity_key.to_base58_string(),
            owner: node.owner.clone(),
            node_type: NodeType::Gateway,
        }
    }
}

impl Display for TestedNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} (owned by {})", self.identity, self.owner)
    }
}

pub(crate) struct PreparedPackets {
    /// All packets that are going to get sent during the test as well as the gateways through
    /// which they ought to be sent.
    pub(super) packets: Vec<GatewayPackets>,

    /// Vector containing list of public keys and owners of all nodes mixnodes being tested.
    pub(super) tested_mixnodes: Vec<TestedNode>,

    /// Vector containing list of public keys and owners of all gateways being tested.
    pub(super) tested_gateways: Vec<TestedNode>,

    /// All mixnodes that failed to get parsed correctly or were not version compatible.
    /// They will be marked to the validator as being down for the test.
    pub(super) invalid_mixnodes: Vec<InvalidNode>,

    /// All gateways that failed to get parsed correctly or were not version compatible.
    /// They will be marked to the validator as being down for the test.
    pub(super) invalid_gateways: Vec<InvalidNode>,
}

pub(crate) struct PacketPreparer {
    system_version: String,
    chunker: Option<Chunker>,
    validator_cache: ValidatorCache,

    /// Number of test packets sent to each node
    per_node_test_packets: usize,

    // TODO: security:
    // in the future we should really create unique set of keys every time otherwise
    // gateways might recognise our "test" keys and take special care to always forward those packets
    // even if otherwise they are malicious.
    self_public_identity: identity::PublicKey,
    self_public_encryption: encryption::PublicKey,
}

impl PacketPreparer {
    pub(crate) fn new(
        system_version: &str,
        validator_cache: ValidatorCache,
        per_node_test_packets: usize,
        self_public_identity: identity::PublicKey,
        self_public_encryption: encryption::PublicKey,
    ) -> Self {
        PacketPreparer {
            system_version: system_version.to_owned(),
            chunker: None,
            validator_cache,
            per_node_test_packets,
            self_public_identity,
            self_public_encryption,
        }
    }

    async fn wrap_test_packet(
        &mut self,
        packet: &TestPacket,
        topology: &NymTopology,
        packet_recipient: Recipient,
    ) -> MixPacket {
        // this should be done only once. We can't really do it at construction time
        // as there's no sane Default for Recipient
        if self.chunker.is_none() {
            self.chunker = Some(Chunker::new(packet_recipient));
        }
        let mut mix_packets = self
            .chunker
            .as_mut()
            .unwrap()
            .prepare_packets_from(packet.to_bytes(), topology, packet_recipient)
            .await;
        assert_eq!(
            mix_packets.len(),
            1,
            "Our test packets data is longer than a single sphinx packet!"
        );

        mix_packets.pop().unwrap()
    }

    pub(crate) async fn wait_for_validator_cache_initial_values(&self, minimum_full_routes: usize) {
        // wait for the cache to get initialised
        self.validator_cache.wait_for_initial_values().await;

        // now wait for at least `minimum_full_routes` mixnodes per layer and `minimum_full_routes` gateway to be online
        info!("Waiting for minimal topology to be online");
        let initialisation_backoff = Duration::from_secs(30);
        loop {
            let gateways = self.validator_cache.gateways_all().await;
            let mixnodes = self.validator_cache.mixnodes_all().await;

            if gateways.len() < minimum_full_routes {
                info!(
                    "Minimal topology is still not online. Going to check again in {:?}",
                    initialisation_backoff
                );
                tokio::time::sleep(initialisation_backoff).await;
                continue;
            }

            let mut layered_mixes = HashMap::new();
            for mix in mixnodes {
                let layer = mix.layer;
                let mixes = layered_mixes.entry(layer).or_insert_with(Vec::new);
                mixes.push(mix)
            }

            // we remove the entries as this gives us the ownership and thus we can unwrap to default value
            // which makes the code slightly nicer without having to deal with options
            let layer1 = layered_mixes.remove(&Layer::One).unwrap_or_default();
            let layer2 = layered_mixes.remove(&Layer::Two).unwrap_or_default();
            let layer3 = layered_mixes.remove(&Layer::Three).unwrap_or_default();

            if layer1.len() >= minimum_full_routes
                && layer2.len() >= minimum_full_routes
                && layer3.len() >= minimum_full_routes
            {
                break;
            }

            info!(
                "Minimal topology is still not online. Going to check again in {:?}",
                initialisation_backoff
            );
            tokio::time::sleep(initialisation_backoff).await;
        }
    }

    async fn all_mixnodes_and_gateways(&self) -> (Vec<MixNodeBond>, Vec<GatewayBond>) {
        info!("Obtaining network topology...");

        let mixnodes = self.validator_cache.mixnodes_all().await;
        let gateways = self.validator_cache.gateways_all().await;

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
        let (mixnodes, gateways) = self.all_mixnodes_and_gateways().await;
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
            None
        } else {
            trace!("Generating test routes...");
            let mut routes = Vec::new();
            for i in 0..most_available {
                let node_1 = match self.try_parse_mix_bond(rand_l1[i]) {
                    Ok(node) => node,
                    Err(id) => {
                        blacklist.insert(id);
                        continue;
                    }
                };

                let node_2 = match self.try_parse_mix_bond(rand_l2[i]) {
                    Ok(node) => node,
                    Err(id) => {
                        blacklist.insert(id);
                        continue;
                    }
                };

                let node_3 = match self.try_parse_mix_bond(rand_l3[i]) {
                    Ok(node) => node,
                    Err(id) => {
                        blacklist.insert(id);
                        continue;
                    }
                };

                let gateway = match self.try_parse_gateway_bond(rand_gateways[i]) {
                    Ok(node) => node,
                    Err(id) => {
                        blacklist.insert(id);
                        continue;
                    }
                };

                routes.push(TestRoute::new(
                    rng.gen(),
                    &self.system_version,
                    node_1,
                    node_2,
                    node_3,
                    gateway,
                ))
            }
            info!("{:?}", routes);
            Some(routes)
        }
    }

    fn create_packet_sender(&self, gateway: &gateway::Node) -> Recipient {
        Recipient::new(
            self.self_public_identity,
            self.self_public_encryption,
            gateway.identity_key,
        )
    }

    pub(crate) async fn prepare_test_route_viability_packets(
        &mut self,
        route: &TestRoute,
        num: usize,
    ) -> GatewayPackets {
        let mut mix_packets = Vec::with_capacity(num);
        let test_packet = route.self_test_packet();
        let recipient = self.create_packet_sender(route.gateway());
        for _ in 0..num {
            let mix_packet = self
                .wrap_test_packet(&test_packet, route.topology(), recipient)
                .await;
            mix_packets.push(mix_packet)
        }

        GatewayPackets::new(
            route.gateway_clients_address(),
            route.gateway_identity(),
            route.gateway_owner(),
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
                invalid_nodes.push(InvalidNode::Malformed(
                    mixnode.mix_node.identity_key,
                    mixnode.owner,
                ));
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
                invalid_nodes.push(InvalidNode::Malformed(
                    gateway.gateway.identity_key,
                    gateway.owner,
                ));
            }
        }
        (parsed_nodes, invalid_nodes)
    }

    pub(super) async fn prepare_test_packets(
        &mut self,
        test_nonce: u64,
        test_routes: &[TestRoute],
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
            let recipient = self.create_packet_sender(test_route.gateway());
            let gateway_identity = test_route.gateway_identity();
            let gateway_address = test_route.gateway_clients_address();
            let gateway_owner = test_route.gateway_owner();

            // it's actually going to be a tiny bit more due to gateway testing, but it's a good enough approximation
            let mut mix_packets = Vec::with_capacity(mixnodes.len() * self.per_node_test_packets);

            // and for each mixnode...
            for mixnode in &mixnodes {
                let test_packet = TestPacket::from_mixnode(mixnode, test_route.id(), test_nonce);
                let topology = test_route.substitute_mix(mixnode);
                // produce n mix packets
                for _ in 0..self.per_node_test_packets {
                    let mix_packet = self
                        .wrap_test_packet(&test_packet, &topology, recipient)
                        .await;
                    mix_packets.push(mix_packet);
                }
            }

            let gateway_packets = all_gateway_packets
                .entry(gateway_identity.to_bytes())
                .or_insert_with(|| {
                    GatewayPackets::empty(gateway_address, gateway_identity, gateway_owner)
                });
            gateway_packets.push_packets(mix_packets);

            // and for each gateway...
            for gateway in &gateways {
                let mut gateway_mix_packets = Vec::new();
                let test_packet = TestPacket::from_gateway(gateway, test_route.id(), test_nonce);
                let gateway_identity = gateway.identity_key;
                let gateway_address = gateway.clients_address();
                let gateway_owner = gateway.owner.clone();
                let recipient = self.create_packet_sender(gateway);
                let topology = test_route.substitute_gateway(gateway);
                // produce n mix packets
                for _ in 0..self.per_node_test_packets {
                    let mix_packet = self
                        .wrap_test_packet(&test_packet, &topology, recipient)
                        .await;
                    gateway_mix_packets.push(mix_packet);
                }

                // and push it into existing struct (if it's a "core" gateway being tested against another route)
                // or create a new one
                let gateway_packets = all_gateway_packets
                    .entry(gateway_identity.to_bytes())
                    .or_insert_with(|| {
                        GatewayPackets::empty(gateway_address, gateway_identity, gateway_owner)
                    });
                gateway_packets.push_packets(gateway_mix_packets);
            }
        }

        // convert our hashmap back into a vec
        let packets = all_gateway_packets.into_iter().map(|(_, v)| v).collect();

        PreparedPackets {
            packets,
            tested_mixnodes,
            tested_gateways,
            invalid_mixnodes,
            invalid_gateways,
        }
    }
}
