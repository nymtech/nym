// Copyright 2020 Nym Technologies SA
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
use crate::gateways_reader::GatewayChannel;
use crate::run_info::{GatewaysInfo, MixesInfo, RunInfo, TestRunUpdate, TestRunUpdateSender};
use crate::test_packet::{IpVersion, TestPacket};
use crate::tested_network::{TestGateway, TestMix, TestedNetwork};
use crypto::asymmetric::identity;
use futures::channel::mpsc;
use gateway_client::error::GatewayClientError;
use gateway_client::{GatewayClient, MixnetMessageSender};
use gateway_requests::registration::handshake::SharedKeys;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::forwarding::packet::MixPacket;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time;
use tokio::sync::Semaphore;
use topology::{gateway, mix, NymTopology};
use validator_client::models::gateway::RegisteredGateway;
use validator_client::models::mixnode::RegisteredMix;
use validator_client::models::topology::Topology;
use validator_client::ValidatorClientError;

#[derive(Debug)]
pub(crate) enum PacketSenderError {
    ValidatorError(ValidatorClientError),
    GatewayError(GatewayClientError),
}

impl From<GatewayClientError> for PacketSenderError {
    fn from(err: GatewayClientError) -> Self {
        PacketSenderError::GatewayError(err)
    }
}

pub struct PacketSenderOld {
    chunker: Chunker,
    local_identity: Arc<identity::KeyPair>,
    validator_client: Arc<validator_client::Client>,
    tested_network: TestedNetwork,
    test_run_sender: TestRunUpdateSender,
    nonce: u64,

    // note: in theory it can allow gateways to "always forward traffic from the monitor client"
    // but in practice it's so unlikely anyone would be bothered (at this stage)
    active_gateway_clients: HashMap<String, GatewayClient>,

    key_cache: HashMap<String, Arc<SharedKeys>>,
}

struct GatewaySender {
    gateway: gateway::Node,
    message_sender: MixnetMessageSender,
    packets: [TestPacket; 2],
}

// TODO: move into different file or something
enum UpdatedGatewayClient {
    Fresh(GatewayClient),
    Dead,
}

impl PacketSenderOld {
    pub(crate) fn new(
        validator_client: Arc<validator_client::Client>,
        tested_network: TestedNetwork,
        self_address: Recipient,
        test_run_sender: TestRunUpdateSender,
    ) -> Self {
        todo!()
        // PacketSender {
        //     chunker: Chunker::new(self_address),
        //     validator_client,
        //     tested_network,
        //     test_run_sender,
        //     nonce: 0,
        // }
    }

    fn check_version_compatibility(&self, mix_version: &str) -> bool {
        let semver_compatibility = version_checker::is_minor_version_compatible(
            mix_version,
            self.tested_network.system_version(),
        );

        if semver_compatibility {
            // this can't fail as we know it's semver compatible
            let version = version_checker::parse_version(mix_version).unwrap();
            if version.major >= 1 {
                return true;
            }
            if version.major == 0 && version.minor >= 10 {
                return true;
            }
            if version.minor >= 9 && version.patch >= 2 {
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn make_test_mix(&self, mix: RegisteredMix) -> TestMix {
        // the reason for that conversion is that I want to operate on concrete types
        // rather than on "String" everywhere and also this way we remove obviously wrong
        // mixnodes where somebody is sending bullshit presence data.
        let mix_id = mix.identity();
        let mix: Result<mix::Node, _> = mix.try_into();
        match mix {
            Err(err) => {
                error!("mix {} is malformed - {:?}", mix_id, err);
                TestMix::MalformedMix(mix_id)
            }
            Ok(mix) => {
                if self.check_version_compatibility(&mix.version) {
                    let v4_test_packet =
                        TestPacket::new(mix.identity_key, IpVersion::V4, self.nonce);
                    let v6_test_packet =
                        TestPacket::new(mix.identity_key, IpVersion::V6, self.nonce);

                    TestMix::ValidMix(mix, [v4_test_packet, v6_test_packet])
                } else {
                    TestMix::IncompatibleMix(mix)
                }
            }
        }
    }

    fn make_test_gateway(&self, gateway: RegisteredGateway) -> TestGateway {
        // the reason for that conversion is that I want to operate on concrete types
        // rather than on "String" everywhere and also this way we remove obviously wrong
        // gateways where somebody is sending bullshit presence data.
        let gate_id = gateway.identity();
        let gateway: Result<gateway::Node, _> = gateway.try_into();

        match gateway {
            Err(err) => {
                error!("gateway {} is malformed - {:?}", gate_id, err);
                TestGateway::MalformedGateway(gate_id)
            }
            Ok(gateway) => {
                if self.check_version_compatibility(&gateway.version) {
                    let v4_test_packet =
                        TestPacket::new(gateway.identity_key, IpVersion::V4, self.nonce);
                    let v6_test_packet =
                        TestPacket::new(gateway.identity_key, IpVersion::V6, self.nonce);

                    TestGateway::ValidGateway(gateway, [v4_test_packet, v6_test_packet])
                } else {
                    TestGateway::IncompatibleGateway(gateway)
                }
            }
        }
    }

    async fn get_test_nodes(&self) -> Result<(Vec<TestMix>, Vec<TestGateway>), PacketSenderError> {
        let topology = self
            .validator_client
            .get_topology()
            .await
            .map_err(PacketSenderError::ValidatorError)?;

        let test_mixes = topology
            .mix_nodes
            .into_iter()
            .map(|mix| self.make_test_mix(mix))
            .collect();

        let test_gateways = topology
            .gateways
            .into_iter()
            .map(|gateway| self.make_test_gateway(gateway))
            .collect();

        Ok((test_mixes, test_gateways))
    }

    async fn get_test_mixes(&self) -> Result<Vec<TestMix>, PacketSenderError> {
        Ok(self
            .validator_client
            .get_topology()
            .await
            .map_err(PacketSenderError::ValidatorError)?
            .mix_nodes
            .into_iter()
            .map(|mix| self.make_test_mix(mix))
            .collect())
    }

    fn prepare_mixes(&self, test_mixes: &[TestMix]) -> MixesInfo {
        let num_valid = test_mixes.iter().filter(|mix| mix.is_valid()).count();
        let mut test_packets = Vec::with_capacity(num_valid * 2);
        let mut malformed_mixes = Vec::new();
        let mut incompatible_mixes = Vec::new();

        for test_mix in test_mixes {
            match test_mix {
                TestMix::ValidMix(.., mix_test_packets) => {
                    test_packets.push(mix_test_packets[0]);
                    test_packets.push(mix_test_packets[1]);
                }
                TestMix::MalformedMix(pub_key) => malformed_mixes.push(pub_key.clone()),
                TestMix::IncompatibleMix(mix) => incompatible_mixes
                    .push((mix.identity_key.to_base58_string(), mix.version.clone())),
            }
        }

        MixesInfo {
            mix_test_packets: test_packets,
            malformed_mixes,
            incompatible_mixes,
        }
    }

    // TODO: perhaps take gateways by value?
    fn prepare_gateways(
        &self,
        test_gateways: &[TestGateway],
    ) -> (GatewaysInfo, Vec<GatewaySender>) {
        todo!()
        // let num_valid = test_gateways
        //     .iter()
        //     .filter(|gateway| gateway.is_valid())
        //     .count();
        //
        // let mut gateway_senders = Vec::with_capacity(num_valid);
        // let mut tested_gateways = Vec::with_capacity(num_valid);
        //
        // let mut malformed_gateways = Vec::new();
        // let mut incompatible_gateways = Vec::new();
        //
        // for test_gateway in test_gateways {
        //     match test_gateway {
        //         TestGateway::ValidGateway(gateway, gateway_test_packets) => {
        //             let (sender, receiver) = mpsc::unbounded();
        //             let gateway_channel = GatewayChannel::new(gateway.identity_key, receiver);
        //
        //             tested_gateways.push((gateway_channel, *gateway_test_packets));
        //             gateway_senders.push(GatewaySender {
        //                 gateway: gateway.clone(),
        //                 message_sender: sender,
        //                 packets: *gateway_test_packets,
        //             });
        //         }
        //         TestGateway::MalformedGateway(pub_key) => malformed_gateways.push(pub_key.clone()),
        //         TestGateway::IncompatibleGateway(gateway) => incompatible_gateways.push((
        //             gateway.identity_key.to_base58_string(),
        //             gateway.version.clone(),
        //         )),
        //     }
        // }
        //
        // (
        //     GatewaysInfo {
        //         tested_gateways,
        //         malformed_gateways,
        //         incompatible_gateways,
        //     },
        //     gateway_senders,
        // )
    }

    fn prepare_run_info(&self, test_mixes: &[TestMix], test_gateways: &[TestGateway]) -> RunInfo {
        todo!()

        // let num_valid = test_mixes.iter().filter(|mix| mix.is_valid()).count();
        // let mut test_packets = Vec::with_capacity(num_valid * 2);
        // let mut malformed_mixes = Vec::new();
        // let mut incompatible_mixes = Vec::new();
        //
        // for test_mix in test_mixes {
        //     match test_mix {
        //         TestMix::ValidMix(.., mix_test_packets) => {
        //             test_packets.push(mix_test_packets[0]);
        //             test_packets.push(mix_test_packets[1]);
        //         }
        //         TestMix::MalformedMix(pub_key) => malformed_mixes.push(pub_key.clone()),
        //         TestMix::IncompatibleMix(mix) => incompatible_mixes
        //             .push((mix.identity_key.to_base58_string(), mix.version.clone())),
        //     }
        // }
        // RunInfo {
        //     nonce: self.nonce,
        //     mix_test_packets: test_packets,
        //     malformed_mixes,
        //     incompatible_mixes,
        // }
    }

    async fn prepare_node_mix_packets(
        &mut self,
        mixnode: mix::Node,
        test_packets: [TestPacket; 2],
    ) -> Vec<MixPacket> {
        let mut packets = Vec::with_capacity(2);
        // for test_packet in test_packets.iter() {
        //     let topology_to_test = self
        //         .tested_network
        //         .substitute_mix(mixnode.clone(), test_packet.ip_version());
        //     let mix_message = test_packet.to_bytes();
        //     let mut mix_packet = self
        //         .chunker
        //         .prepare_packets(mix_message, &topology_to_test)
        //         .await;
        //     debug_assert_eq!(mix_packet.len(), 1);
        //     packets.push(mix_packet.pop().unwrap());
        // }
        packets
    }

    async fn prepare_mix_packets(&mut self, test_mixes: Vec<TestMix>) -> Vec<MixPacket> {
        let num_valid = test_mixes.iter().filter(|mix| mix.is_valid()).count();
        let mut mix_packets = Vec::with_capacity(2 * num_valid);

        for test_mix in test_mixes {
            match test_mix {
                TestMix::ValidMix(mixnode, test_packets) => {
                    let mut node_mix_packets =
                        self.prepare_node_mix_packets(mixnode, test_packets).await;
                    mix_packets.append(&mut node_mix_packets);
                }
                _ => continue,
            }
        }
        mix_packets
    }

    async fn send_messages(
        &mut self,
        mix_packets: Vec<MixPacket>,
    ) -> Result<(), PacketSenderError> {
        todo!()
        // self.tested_network.send_messages(mix_packets).await?;
        // Ok(())
    }

    async fn prepare_gateway_packets(
        &mut self,
        gateway: gateway::Node,
        test_packets: [TestPacket; 2],
    ) -> Vec<MixPacket> {
        let mut packets = Vec::with_capacity(2);
        // for test_packet in test_packets.iter() {
        //     let topology_to_test = self
        //         .tested_network
        //         .substitute_gateway(gateway.clone(), test_packet.ip_version());
        //     let mix_message = test_packet.to_bytes();
        //     let mut mix_packet = self
        //         .chunker
        //         .prepare_packets(mix_message, &topology_to_test)
        //         .await;
        //     debug_assert_eq!(mix_packet.len(), 1);
        //     packets.push(mix_packet.pop().unwrap());
        // }
        packets
    }

    // todo: put arguments in new struct like NewClientMaterial or something and then put
    // it all in an option
    async fn send_to_tested_gateway(
        gateway_sender: GatewaySender,
        gateway_client: Option<GatewayClient>,
        cached_keys: Option<Arc<SharedKeys>>,
        local_identity: Option<Arc<identity::KeyPair>>,
        packets: Vec<MixPacket>,
    ) -> Option<UpdatedGatewayClient> {
        let (ack_sender, _ack_receiver) = mpsc::unbounded();
        // temp. to be moved to config or something
        let timeout = time::Duration::from_millis(500);

        let gateway = gateway_sender.gateway;
        let gateway_id = gateway.identity_key.to_base58_string();

        let mut is_fresh_client = false;

        let mut gateway_client = if let Some(client) = gateway_client {
            client
        } else {
            is_fresh_client = true;
            gateway_client::GatewayClient::new(
                gateway.client_listener,
                local_identity.expect("TODO: this will be removed in favour of single struct"),
                gateway.identity_key,
                cached_keys,
                gateway_sender.message_sender,
                ack_sender,
                timeout,
            )
        };

        match gateway_client.authenticate_and_start().await {
            Ok(_) => {
                if let Err(err) = gateway_client.batch_send_mix_packets(packets).await {
                    warn!(
                        "failed to send mix packets to - {:?} - {:?}",
                        gateway_id, err
                    );
                    if is_fresh_client {
                        None
                    } else {
                        Some(UpdatedGatewayClient::Dead)
                    }
                } else {
                    if is_fresh_client {
                        Some(UpdatedGatewayClient::Fresh(gateway_client))
                    } else {
                        None
                    }
                }
            }
            Err(err) => {
                warn!("failed to authenticate with {:?} - {:?}", gateway_id, err);
                Some(UpdatedGatewayClient::Dead)
            }
        }
    }

    const MAX_ACTIVE_SENDERS: usize = 5;
    async fn send_to_gateways(&self, gateway_senders: Vec<GatewaySender>) {
        let this = Arc::new(self);
        if gateway_senders.len() > Self::MAX_ACTIVE_SENDERS {
            let mut semaphore = Semaphore::new(Self::MAX_ACTIVE_SENDERS);
        // limit number of concurrent senders with semaphore
        } else {
            for gateway_sender in gateway_senders {
                let this = Arc::clone(&this);
                todo!()
                // tokio::spawn(this.send_to_tested_gateway(gateway_sender));
            }
            // spawn everything at once - don't bother with semaphore
        }
    }

    pub(crate) async fn run_test(&mut self) -> Result<(), PacketSenderError> {
        self.nonce += 1;

        let (test_mixes, test_gateways) = self.get_test_nodes().await?;

        info!(target: "Monitor", "Going to test {} mixes and {} gateways", test_mixes.len(), test_gateways.len());
        let run_info = self.prepare_run_info(&test_mixes, &test_gateways);
        let mix_packets = self.prepare_mix_packets(test_mixes).await;

        if !test_gateways.is_empty() {
            // TODO: maybe some thread pool and join action?
            // attempt connections, etc?
        }

        // run info should also contain channels to gateways

        // inform notifier that we're about to start the test
        self.test_run_sender
            .unbounded_send(TestRunUpdate::StartSending(run_info))
            .expect("notifier has crashed!");

        self.send_messages(mix_packets).await?;

        // inform the notifier we're done sending (so that it should start its timeout)
        self.test_run_sender
            .unbounded_send(TestRunUpdate::DoneSending(self.nonce))
            .expect("notifier has crashed!");

        info!(target: "Monitor", "Waiting for the test run to finish...");
        Ok(())
    }
}
