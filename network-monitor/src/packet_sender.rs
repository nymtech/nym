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
use crate::test_packet::{IpVersion, TestPacket};
use crate::test_run::{RunInfo, TestRunUpdate, TestRunUpdateSender};
use directory_client::presence::mixnodes::MixNodePresence;
use gateway_client::error::GatewayClientError;
use gateway_client::GatewayClient;
use log::*;
use nymsphinx::{
    addressing::{clients::Recipient, nodes::NymNodeRoutingAddress},
    SphinxPacket,
};
use std::convert::TryInto;
use std::sync::Arc;
use topology::{mix, NymTopology};

enum TestMix {
    ValidMix(mix::Node, [TestPacket; 2]),
    MalformedMix(String),
}

impl TestMix {
    fn is_valid(&self) -> bool {
        match self {
            TestMix::ValidMix(..) => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub(crate) enum PacketSenderError {
    DirectoryError(String),
    GatewayError(GatewayClientError),
}

impl From<GatewayClientError> for PacketSenderError {
    fn from(err: GatewayClientError) -> Self {
        PacketSenderError::GatewayError(err)
    }
}

pub struct PacketSender {
    chunker: Chunker,
    directory_client: Arc<directory_client::Client>,
    gateway_client: GatewayClient,
    good_topology: NymTopology,
    test_run_sender: TestRunUpdateSender,
    nonce: u64,
}

impl PacketSender {
    pub(crate) fn new(
        directory_client: Arc<directory_client::Client>,
        good_topology: NymTopology,
        self_address: Recipient,
        gateway_client: GatewayClient,
        test_run_sender: TestRunUpdateSender,
    ) -> Self {
        PacketSender {
            chunker: Chunker::new(self_address),
            directory_client,
            gateway_client,
            good_topology,
            test_run_sender,
            nonce: 0,
        }
    }

    pub(crate) async fn start_gateway_client(&mut self) {
        self.gateway_client
            .authenticate_and_start()
            .await
            .expect("Couldn't authenticate with gateway node.");
    }

    /// Run some initial checks to ensure our subsequent measurements are valid.
    /// For example, we should be able to send ourselves a Sphinx packet (and receive it
    /// via the websocket, which currently fails.
    pub(crate) async fn sanity_check(&mut self) -> Result<(), PacketSenderError> {
        let messages = self
            .chunker
            .prepare_messages(b"hello".to_vec(), &self.good_topology);
        self.send_messages(messages).await
    }

    fn make_test_mix(&self, presence: MixNodePresence) -> TestMix {
        // the reason for that conversion is that I want to operate on concrete types
        // rather than on "String" everywhere and also this way we remove obviously wrong
        // mixnodes where somebody is sending bullshit presence data.
        let mix_id = presence.pub_key.clone();
        let mix: Result<mix::Node, _> = presence.try_into();
        match mix {
            Err(err) => {
                error!("mix {} is malformed - {:?}", mix_id, err);
                TestMix::MalformedMix(mix_id)
            }
            Ok(mix) => {
                let v4_test_packet = TestPacket::new(mix.pub_key, IpVersion::V4, self.nonce);
                let v6_test_packet = TestPacket::new(mix.pub_key, IpVersion::V6, self.nonce);

                TestMix::ValidMix(mix, [v4_test_packet, v6_test_packet])
            }
        }
    }

    async fn get_test_mixes(&self) -> Result<Vec<TestMix>, PacketSenderError> {
        Ok(self
            .directory_client
            .get_topology()
            .await
            .map_err(|err| PacketSenderError::DirectoryError(err.to_string()))?
            .mix_nodes
            .into_iter()
            .map(|presence| self.make_test_mix(presence))
            .collect())
    }

    fn prepare_run_info(&self, test_mixes: &[TestMix]) -> RunInfo {
        let num_valid = test_mixes.iter().filter(|mix| mix.is_valid()).count();
        let mut test_packets = Vec::with_capacity(num_valid * 2);
        let mut malformed_mixes = Vec::with_capacity(test_mixes.len() - num_valid);

        for test_mix in test_mixes {
            match test_mix {
                TestMix::ValidMix(.., mix_test_packets) => {
                    test_packets.push(mix_test_packets[0]);
                    test_packets.push(mix_test_packets[1]);
                }
                TestMix::MalformedMix(pub_key) => malformed_mixes.push(pub_key.clone()),
            }
        }
        RunInfo {
            nonce: self.nonce,
            test_packets,
            malformed_mixes,
        }
    }

    // TODO: don't mind return type, it will be replaced after merge with develop
    fn prepare_mix_packets(
        &mut self,
        test_mixes: Vec<TestMix>,
    ) -> Vec<(NymNodeRoutingAddress, SphinxPacket)> {
        let num_valid = test_mixes.iter().filter(|mix| mix.is_valid()).count();
        let mut mix_packets = Vec::with_capacity(num_valid);

        for test_mix in test_mixes {
            match test_mix {
                TestMix::ValidMix(mixnode, test_packets) => {
                    let mut topology_to_test = self.good_topology.clone();
                    topology_to_test.set_mixes_in_layer(mixnode.layer as u8, vec![mixnode]);

                    let message1 = test_packets[0].to_bytes();
                    let message2 = test_packets[1].to_bytes();
                    let mut packet1 = self.chunker.prepare_messages(message1, &topology_to_test);
                    let mut packet2 = self.chunker.prepare_messages(message2, &topology_to_test);

                    // such short messages MUST BE converted into single sphinx packet
                    assert_eq!(packet1.len(), 1,);
                    assert_eq!(packet2.len(), 1,);
                    mix_packets.push(packet1.pop().unwrap());
                    mix_packets.push(packet2.pop().unwrap());
                }
                _ => continue,
            }
        }
        mix_packets
    }

    async fn send_messages(
        &mut self,
        socket_messages: Vec<(NymNodeRoutingAddress, SphinxPacket)>,
    ) -> Result<(), PacketSenderError> {
        self.gateway_client
            .batch_send_sphinx_packets(socket_messages)
            .await?;
        Ok(())
    }

    pub(crate) async fn run_test(&mut self) -> Result<(), PacketSenderError> {
        self.nonce += 1;

        let test_mixes = self.get_test_mixes().await?;
        let run_info = self.prepare_run_info(&test_mixes);
        let mix_packets = self.prepare_mix_packets(test_mixes);

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
