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
use crate::run_info::{RunInfo, TestRunUpdate, TestRunUpdateSender};
use crate::test_packet::{IpVersion, TestPacket};
use crate::tested_network::{TestMix, TestedNetwork};
use gateway_client::error::GatewayClientError;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::forwarding::packet::MixPacket;
use std::convert::TryInto;
use std::sync::Arc;
use topology::mix;
use validator_client::models::mixnode::RegisteredMix;
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

pub struct PacketSender {
    chunker: Chunker,
    validator_client: Arc<validator_client::Client>,
    tested_network: TestedNetwork,
    test_run_sender: TestRunUpdateSender,
    nonce: u64,
}

impl PacketSender {
    pub(crate) fn new(
        validator_client: Arc<validator_client::Client>,
        tested_network: TestedNetwork,
        self_address: Recipient,
        test_run_sender: TestRunUpdateSender,
    ) -> Self {
        PacketSender {
            chunker: Chunker::new(self_address),
            validator_client,
            tested_network,
            test_run_sender,
            nonce: 0,
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
                if version_checker::is_minor_version_compatible(
                    &mix.version,
                    self.tested_network.system_version(),
                ) {
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

    fn prepare_run_info(&self, test_mixes: &[TestMix]) -> RunInfo {
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
        RunInfo {
            nonce: self.nonce,
            test_packets,
            malformed_mixes,
            incompatible_mixes,
        }
    }

    async fn prepare_node_mix_packets(
        &mut self,
        mixnode: mix::Node,
        test_packets: [TestPacket; 2],
    ) -> Vec<MixPacket> {
        let mut packets = Vec::with_capacity(2);
        for test_packet in test_packets.iter() {
            let topology_to_test = self
                .tested_network
                .substitute_node(mixnode.clone(), test_packet.ip_version());
            let mix_message = test_packet.to_bytes();
            let mut mix_packet = self
                .chunker
                .prepare_messages(mix_message, &topology_to_test)
                .await;
            debug_assert_eq!(mix_packet.len(), 1);
            packets.push(mix_packet.pop().unwrap());
        }
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
        self.tested_network.send_messages(mix_packets).await?;
        Ok(())
    }

    pub(crate) async fn run_test(&mut self) -> Result<(), PacketSenderError> {
        self.nonce += 1;

        let test_mixes = self.get_test_mixes().await?;
        info!(target: "Monitor", "Going to test {} mixes", test_mixes.len());
        let run_info = self.prepare_run_info(&test_mixes);
        let mix_packets = self.prepare_mix_packets(test_mixes).await;

        // inform notifier that we're about to start the test
        self.test_run_sender
            .unbounded_send(TestRunUpdate::StartSending(run_info))
            .expect("notifier has crashed!");

        self.send_messages(mix_packets).await?;

        // inform the notifier we're done sending (so that it should start its timeout)
        self.test_run_sender
            .unbounded_send(TestRunUpdate::DoneSending(self.nonce))
            .expect("notifier has crashed!");

        Ok(())
    }
}
