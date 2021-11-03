// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cache::ValidatorCache;
use crate::network_monitor::chunker::Chunker;
use crate::network_monitor::monitor::sender::GatewayPackets;
use crate::network_monitor::test_packet::{NodeType, TestPacket};
use crate::network_monitor::tested_network::TestedNetwork;
use crypto::asymmetric::{encryption, identity};
use log::{info, warn};
use mixnet_contract::{GatewayBond, MixNodeBond};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::forwarding::packet::MixPacket;
use std::convert::TryInto;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;
use topology::{gateway, mix};

// declared type aliases for easier code reasoning
type Version = String;
type Id = String;
type Owner = String;

#[derive(Clone)]
pub(crate) enum InvalidNode {
    OutdatedMix(Id, Owner, Version),
    MalformedMix(Id, Owner),
    OutdatedGateway(Id, Owner, Version),
    MalformedGateway(Id, Owner),
}

impl Display for InvalidNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InvalidNode::OutdatedMix(id, owner, version) => {
                write!(
                    f,
                    "Mixnode {} (v {}) owned by {} is outdated",
                    id, version, owner
                )
            }
            InvalidNode::MalformedMix(id, owner) => {
                write!(f, "Mixnode {} owner by {} is malformed", id, owner)
            }
            InvalidNode::OutdatedGateway(id, owner, version) => {
                write!(
                    f,
                    "Gateway {} (v {}) owned by {} is outdated",
                    id, version, owner
                )
            }
            InvalidNode::MalformedGateway(id, owner) => {
                write!(f, "Gateway {} owned by {} is malformed", id, owner)
            }
        }
    }
}

enum PreparedNode {
    TestedGateway(gateway::Node, [TestPacket; 2]),
    TestedMix(mix::Node, [TestPacket; 2]),
    Invalid(InvalidNode),
}

#[derive(Eq, PartialEq, Debug, Hash, Clone)]
pub(crate) struct TestedNode {
    pub(crate) identity: String,
    pub(crate) owner: String,
    pub(crate) node_type: NodeType,
}

impl TestedNode {
    pub(crate) fn new_mix(identity: String, owner: String) -> Self {
        TestedNode {
            identity,
            owner,
            node_type: NodeType::Mixnode,
        }
    }

    pub(crate) fn new_gateway(identity: String, owner: String) -> Self {
        TestedNode {
            identity,
            owner,
            node_type: NodeType::Gateway,
        }
    }

    pub(crate) fn from_raw_mix<S1, S2>(identity: S1, owner: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        TestedNode {
            identity: identity.into(),
            owner: owner.into(),
            node_type: NodeType::Mixnode,
        }
    }

    pub(crate) fn from_raw_gateway<S1, S2>(identity: S1, owner: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        TestedNode {
            identity: identity.into(),
            owner: owner.into(),
            node_type: NodeType::Gateway,
        }
    }

    pub(crate) fn is_gateway(&self) -> bool {
        self.node_type == NodeType::Gateway
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

    /// Vector containing list of public keys and owners of all nodes (mixnodes and gateways) being tested.
    /// We do not need to specify the exact test parameters as for each of them we expect to receive
    /// two packets back: ipv4 and ipv6 regardless of which gateway they originate.
    pub(super) tested_nodes: Vec<TestedNode>,

    /// All nodes that failed to get parsed correctly. They will be marked to the validator as being
    /// down on ipv4 and ipv6.
    pub(super) invalid_nodes: Vec<InvalidNode>,
}

pub(crate) struct PacketPreparer {
    chunker: Chunker,
    validator_cache: ValidatorCache,
    tested_network: TestedNetwork,

    // currently all test MIXNODE packets are sent via the same gateway
    test_mixnode_sender: Recipient,

    // keys required to create sender of any other gateway
    self_public_identity: identity::PublicKey,
    self_public_encryption: encryption::PublicKey,
}

impl PacketPreparer {
    pub(crate) fn new(
        validator_cache: ValidatorCache,
        tested_network: TestedNetwork,
        test_mixnode_sender: Recipient,
        self_public_identity: identity::PublicKey,
        self_public_encryption: encryption::PublicKey,
    ) -> Self {
        PacketPreparer {
            chunker: Chunker::new(test_mixnode_sender),
            validator_cache,
            tested_network,
            test_mixnode_sender,
            self_public_identity,
            self_public_encryption,
        }
    }

    pub(crate) async fn wait_for_validator_cache_initial_values(&self) {
        // wait for the cache to get initialised
        self.validator_cache.wait_for_initial_values().await;

        // now wait for our "good" topology to be online
        info!("Waiting for 'good' topology to be online");
        let initialisation_backoff = Duration::from_secs(30);
        loop {
            let gateways = self.validator_cache.gateways().await;
            let mixnodes = self.validator_cache.mixnodes().await;
            if self
                .tested_network
                .is_online(&mixnodes.into_inner(), &gateways.into_inner())
            {
                break;
            } else {
                info!(
                    "Our 'good' topology is still not offline. Going to check again in {:?}",
                    initialisation_backoff
                );
                tokio::time::sleep(initialisation_backoff).await;
            }
        }
    }

    async fn get_demanded_nodes(&self) -> (Vec<MixNodeBond>, Vec<GatewayBond>) {
        info!(target: "Monitor", "Obtaining network topology...");

        let mixnodes = self.validator_cache.demanded_mixnodes().await.into_inner();
        let gateways = self.validator_cache.gateways().await.into_inner();

        info!(target: "Monitor", "Obtained network topology");

        (mixnodes, gateways)
    }

    fn check_version_compatibility(&self, mix_version: &str) -> bool {
        let semver_compatibility = version_checker::is_minor_version_compatible(
            mix_version,
            self.tested_network.system_version(),
        );

        if semver_compatibility {
            // this can't fail as we know it's semver compatible
            let version = version_checker::parse_version(mix_version).unwrap();

            // check if it's at least 0.9.2 - reject anything below it due to significant bugs present
            // if it's 1.Y.Z it's definitely >= 0.9.2
            if version.major >= 1 {
                return true;
            }
            // if it's 0.10.Z it's definitely >= 0.9.2
            if version.major == 0 && version.minor >= 10 {
                return true;
            }
            // if it's 0.9.Z, ensure Z >= 2
            version.minor == 9 && version.patch >= 2
        } else {
            false
        }
    }

    fn mix_into_prepared_node(&self, nonce: u64, mixnode_bond: &MixNodeBond) -> PreparedNode {
        if !self.check_version_compatibility(&mixnode_bond.mix_node().version) {
            return PreparedNode::Invalid(InvalidNode::OutdatedMix(
                mixnode_bond.mix_node().identity_key.clone(),
                mixnode_bond.owner.to_string(),
                mixnode_bond.mix_node().version.clone(),
            ));
        }
        match TryInto::<mix::Node>::try_into(mixnode_bond) {
            Ok(mix) => {
                let v4_packet = TestPacket::new_v4(
                    mix.identity_key,
                    mix.owner.clone(),
                    nonce,
                    NodeType::Mixnode,
                );
                let v6_packet = TestPacket::new_v6(
                    mix.identity_key,
                    mix.owner.clone(),
                    nonce,
                    NodeType::Mixnode,
                );
                PreparedNode::TestedMix(mix, [v4_packet, v6_packet])
            }
            Err(err) => {
                warn!(
                    target: "Bad node",
                    "Mix {} is malformed - {}",
                    mixnode_bond.mix_node().identity_key,
                    err
                );
                PreparedNode::Invalid(InvalidNode::MalformedMix(
                    mixnode_bond.mix_node().identity_key.clone(),
                    mixnode_bond.owner.to_string(),
                ))
            }
        }
    }

    fn gateway_into_prepared_node(&self, nonce: u64, gateway_bond: &GatewayBond) -> PreparedNode {
        if !self.check_version_compatibility(&gateway_bond.gateway().version) {
            return PreparedNode::Invalid(InvalidNode::OutdatedGateway(
                gateway_bond.gateway().identity_key.clone(),
                gateway_bond.owner.to_string(),
                gateway_bond.gateway().version.clone(),
            ));
        }
        match TryInto::<gateway::Node>::try_into(gateway_bond) {
            Ok(gateway) => {
                let v4_packet = TestPacket::new_v4(
                    gateway.identity_key,
                    gateway.owner.clone(),
                    nonce,
                    NodeType::Gateway,
                );
                let v6_packet = TestPacket::new_v6(
                    gateway.identity_key,
                    gateway.owner.clone(),
                    nonce,
                    NodeType::Gateway,
                );
                PreparedNode::TestedGateway(gateway, [v4_packet, v6_packet])
            }
            Err(err) => {
                warn!(
                    target: "Bad node",
                    "gateway {} is malformed - {:?}",
                    gateway_bond.gateway().identity_key,
                    err
                );
                PreparedNode::Invalid(InvalidNode::MalformedGateway(
                    gateway_bond.gateway().identity_key.clone(),
                    gateway_bond.owner.to_string(),
                ))
            }
        }
    }

    fn prepare_mixnodes(&self, nonce: u64, nodes: &[MixNodeBond]) -> Vec<PreparedNode> {
        nodes
            .iter()
            .map(|mix| self.mix_into_prepared_node(nonce, mix))
            .collect()
    }

    fn prepare_gateways(&self, nonce: u64, nodes: &[GatewayBond]) -> Vec<PreparedNode> {
        nodes
            .iter()
            .map(|gateway| self.gateway_into_prepared_node(nonce, gateway))
            .collect()
    }

    fn tested_nodes(&self, nodes: &[PreparedNode]) -> Vec<TestedNode> {
        nodes
            .iter()
            .filter_map(|node| match node {
                PreparedNode::TestedGateway(gateway, _) => Some(TestedNode::new_gateway(
                    gateway.identity_key.to_base58_string(),
                    gateway.owner.clone(),
                )),
                PreparedNode::TestedMix(mix, _) => Some(TestedNode::new_mix(
                    mix.identity_key.to_base58_string(),
                    mix.owner.clone(),
                )),
                PreparedNode::Invalid(..) => None,
            })
            .collect()
    }

    fn create_packet_sender(&self, gateway: &gateway::Node) -> Recipient {
        Recipient::new(
            self.self_public_identity,
            self.self_public_encryption,
            gateway.identity_key,
        )
    }

    async fn create_mixnode_mix_packets(
        &mut self,
        mixes: Vec<PreparedNode>,
        invalid: &mut Vec<InvalidNode>,
    ) -> Vec<MixPacket> {
        // all of the mixnode mix packets are going to get sent via our one 'main' gateway
        // TODO: in the future this should probably be changed...

        // this might be slightly overestimating the number of packets we are going to produce,
        // however, it should be negligible as we don't expect to ever see high number of
        // invalid nodes (and even if we do see them, they shouldn't persist for long)
        let mut packets = Vec::with_capacity(mixes.len() * 2);

        for mix in mixes.into_iter() {
            match mix {
                PreparedNode::TestedMix(node, test_packets) => {
                    for test_packet in test_packets.iter() {
                        let topology_to_test = self
                            .tested_network
                            .substitute_mix(node.clone(), test_packet.ip_version());
                        let mix_message = test_packet.to_bytes();
                        let mut mix_packet = self
                            .chunker
                            .prepare_packets_from(
                                mix_message,
                                &topology_to_test,
                                self.test_mixnode_sender,
                            )
                            .await;
                        debug_assert_eq!(mix_packet.len(), 1);
                        packets.push(mix_packet.pop().unwrap());
                    }
                }
                PreparedNode::Invalid(node) => invalid.push(node),
                // `prepare_mixnodes` should NEVER return prepared gateways
                _ => unreachable!(),
            }
        }
        packets
    }

    async fn create_gateway_mix_packets(
        &mut self,
        gateways: Vec<PreparedNode>,
        invalid: &mut Vec<InvalidNode>,
    ) -> Vec<GatewayPackets> {
        // again, there might be a slight overestimation here, but in the grand scheme of things
        // it will be negligible
        let mut packets = Vec::with_capacity(gateways.len());

        // unfortunately this can't be done more cleanly with iterators as we require an async call
        for gateway in gateways.into_iter() {
            match gateway {
                PreparedNode::TestedGateway(node, test_packets) => {
                    let mut gateway_packets = Vec::with_capacity(2);
                    for test_packet in test_packets.iter() {
                        let packet_sender = self.create_packet_sender(&node);
                        let topology_to_test = self
                            .tested_network
                            .substitute_gateway(node.clone(), test_packet.ip_version());
                        let mix_message = test_packet.to_bytes();
                        let mut mix_packet = self
                            .chunker
                            .prepare_packets_from(mix_message, &topology_to_test, packet_sender)
                            .await;
                        debug_assert_eq!(mix_packet.len(), 1);

                        gateway_packets.push(mix_packet.pop().unwrap());
                    }
                    packets.push(GatewayPackets::new(
                        node.clients_address(),
                        node.identity_key,
                        gateway_packets,
                    ))
                }
                PreparedNode::Invalid(node) => invalid.push(node),
                // `prepare_gateways` should NEVER return prepared mixnodes
                _ => unreachable!(),
            }
        }

        packets
    }

    pub(super) async fn prepare_test_packets(&mut self, nonce: u64) -> PreparedPackets {
        // only test nodes that are demanded, i.e. that will be rewarded in this epoch.
        let (mixnode_bonds, gateway_bonds) = self.get_demanded_nodes().await;

        let mut invalid_nodes = Vec::new();
        let mixes = self.prepare_mixnodes(nonce, &mixnode_bonds);
        let gateways = self.prepare_gateways(nonce, &gateway_bonds);

        // get the keys of all nodes that will get tested this round
        let tested_nodes: Vec<_> = self
            .tested_nodes(&mixes)
            .into_iter()
            .chain(self.tested_nodes(&gateways).into_iter())
            .collect();

        // those packets are going to go to our 'main' gateway
        let mix_packets = self
            .create_mixnode_mix_packets(mixes, &mut invalid_nodes)
            .await;

        let main_gateway_id = self.tested_network.main_v4_gateway().identity_key;

        let mut gateway_packets = self
            .create_gateway_mix_packets(gateways, &mut invalid_nodes)
            .await;

        // check whether our 'good' gateway is being tested
        if let Some(tested_main_gateway_packets) = gateway_packets
            .iter_mut()
            .find(|gateway| gateway.gateway_address() == main_gateway_id)
        {
            // we are testing the gateway we specified in our 'good' topology
            tested_main_gateway_packets.push_packets(mix_packets);
        } else {
            // we are not testing the gateway from our 'good' topology -> it's probably
            // situation similar to using 'good' qa-topology but testing testnet nodes.
            let main_gateway_packets = GatewayPackets::new(
                self.tested_network.main_v4_gateway().clients_address(),
                main_gateway_id,
                mix_packets,
            );
            gateway_packets.push(main_gateway_packets);
        }

        PreparedPackets {
            packets: gateway_packets,
            tested_nodes,
            invalid_nodes,
        }
    }
}
