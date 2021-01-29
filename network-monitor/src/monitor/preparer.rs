// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::chunker::Chunker;
use crate::monitor::sender::GatewayPackets;
use crate::test_packet::TestPacket;
use crate::tested_network::TestedNetwork;
use crypto::asymmetric::{encryption, identity};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::forwarding::packet::MixPacket;
use std::convert::TryInto;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use topology::{gateway, mix};
use validator_client::models::gateway::RegisteredGateway;
use validator_client::models::mixnode::RegisteredMix;
use validator_client::models::topology::Topology;
use validator_client::ValidatorClientError;

#[derive(Debug)]
pub(super) enum PacketPreparerError {
    ValidatorError(ValidatorClientError),
}

// declared type aliases for easier code reasoning
type Version = String;
type Id = String;

#[derive(Clone)]
pub(crate) enum InvalidNode {
    OutdatedMix(Id, Version),
    MalformedMix(Id),
    OutdatedGateway(Id, Version),
    MalformedGateway(Id),
}

impl Display for InvalidNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InvalidNode::OutdatedMix(id, version) => {
                write!(f, "Mixnode {} (v {}) is outdated", id, version)
            }
            InvalidNode::MalformedMix(id) => write!(f, "Mixnode {} is malformed", id),
            InvalidNode::OutdatedGateway(id, version) => {
                write!(f, "Gateway {} (v {}) is outdated", id, version)
            }
            InvalidNode::MalformedGateway(id) => write!(f, "Gateway {} is malformed", id),
        }
    }
}

enum PreparedNode {
    TestedGateway(gateway::Node, [TestPacket; 2]),
    TestedMix(mix::Node, [TestPacket; 2]),
    Invalid(InvalidNode),
}

pub(crate) struct PreparedPackets {
    /// All packets that are going to get sent during the test as well as the gateways through
    /// which they ought to be sent.
    pub(super) packets: Vec<GatewayPackets>,

    /// Vector containing list of public keys of all nodes (mixnodes and gateways) being tested.
    /// We do not need to specify the exact test parameters as for each of them we expect to receive
    /// two packets back: ipv4 and ipv6 regardless of which gateway they originate.
    pub(super) tested_nodes: Vec<identity::PublicKey>,

    /// All nodes that failed to get parsed correctly. They will be marked to the validator as being
    /// down on ipv4 and ipv6.
    pub(super) invalid_nodes: Vec<InvalidNode>,
}

pub(crate) struct PacketPreparer {
    chunker: Chunker,
    validator_client: Arc<validator_client::Client>,
    tested_network: TestedNetwork,

    // currently all test MIXNODE packets are sent via the same gateway
    test_mixnode_sender: Recipient,

    // keys required to create sender of any other gateway
    self_public_identity: identity::PublicKey,
    self_public_encryption: encryption::PublicKey,
}

impl PacketPreparer {
    pub(crate) fn new(
        validator_client: Arc<validator_client::Client>,
        tested_network: TestedNetwork,
        test_mixnode_sender: Recipient,
        self_public_identity: identity::PublicKey,
        self_public_encryption: encryption::PublicKey,
    ) -> Self {
        PacketPreparer {
            chunker: Chunker::new(test_mixnode_sender),
            validator_client,
            tested_network,
            test_mixnode_sender,
            self_public_identity,
            self_public_encryption,
        }
    }

    async fn get_network_topology(&self) -> Result<Topology, PacketPreparerError> {
        self.validator_client
            .get_topology()
            .await
            .map_err(PacketPreparerError::ValidatorError)
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

    fn mix_into_prepared_node(&self, nonce: u64, registered_mix: &RegisteredMix) -> PreparedNode {
        if !self.check_version_compatibility(registered_mix.version_ref()) {
            return PreparedNode::Invalid(InvalidNode::OutdatedMix(
                registered_mix.identity(),
                registered_mix.version(),
            ));
        }
        match TryInto::<mix::Node>::try_into(registered_mix) {
            Ok(mix) => {
                let v4_packet = TestPacket::new_v4(mix.identity_key, nonce);
                let v6_packet = TestPacket::new_v6(mix.identity_key, nonce);
                PreparedNode::TestedMix(mix, [v4_packet, v6_packet])
            }
            Err(err) => {
                warn!("mix {} is malformed - {:?}", registered_mix.identity(), err);
                PreparedNode::Invalid(InvalidNode::MalformedMix(registered_mix.identity()))
            }
        }
    }

    fn gateway_into_prepared_node(
        &self,
        nonce: u64,
        registered_gateway: &RegisteredGateway,
    ) -> PreparedNode {
        if !self.check_version_compatibility(registered_gateway.version_ref()) {
            return PreparedNode::Invalid(InvalidNode::OutdatedGateway(
                registered_gateway.identity(),
                registered_gateway.version(),
            ));
        }
        match TryInto::<gateway::Node>::try_into(registered_gateway) {
            Ok(gateway) => {
                let v4_packet = TestPacket::new_v4(gateway.identity_key, nonce);
                let v6_packet = TestPacket::new_v6(gateway.identity_key, nonce);
                PreparedNode::TestedGateway(gateway, [v4_packet, v6_packet])
            }
            Err(err) => {
                warn!(
                    "gateway {} is malformed - {:?}",
                    registered_gateway.identity(),
                    err
                );
                PreparedNode::Invalid(InvalidNode::MalformedGateway(registered_gateway.identity()))
            }
        }
    }

    fn prepare_mixnodes(&self, nonce: u64, topology: &Topology) -> Vec<PreparedNode> {
        topology
            .mix_nodes
            .iter()
            .map(|mix| self.mix_into_prepared_node(nonce, mix))
            .collect()
    }

    fn prepare_gateways(&self, nonce: u64, topology: &Topology) -> Vec<PreparedNode> {
        topology
            .gateways
            .iter()
            .map(|gateway| self.gateway_into_prepared_node(nonce, gateway))
            .collect()
    }

    fn tested_nodes(&self, nodes: &[PreparedNode]) -> Vec<identity::PublicKey> {
        nodes
            .iter()
            .filter_map(|node| match node {
                PreparedNode::TestedGateway(gateway, _) => Some(gateway.identity_key),
                PreparedNode::TestedMix(mix, _) => Some(mix.identity_key),
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
                        node.client_listener,
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

    pub(super) async fn prepare_test_packets(
        &mut self,
        nonce: u64,
    ) -> Result<PreparedPackets, PacketPreparerError> {
        let topology = self.get_network_topology().await?;

        let mut invalid_nodes = Vec::new();
        let mixes = self.prepare_mixnodes(nonce, &topology);
        let gateways = self.prepare_gateways(nonce, &topology);

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
                self.tested_network
                    .main_v4_gateway()
                    .client_listener
                    .clone(),
                main_gateway_id,
                mix_packets,
            );
            gateway_packets.push(main_gateway_packets);
        }

        Ok(PreparedPackets {
            packets: gateway_packets,
            tested_nodes,
            invalid_nodes,
        })
    }
}
