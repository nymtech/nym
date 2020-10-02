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

use super::{chunker, good_topology};
use directory_client::presence::mixnodes::MixNodePresence;
use gateway_client::GatewayClient;
use nymsphinx::{
    addressing::{clients::Recipient, nodes::NymNodeRoutingAddress},
    SphinxPacket,
};
use std::sync::Arc;
use topology::NymTopology;

pub struct PacketSender {
    directory_client: Arc<directory_client::Client>,
    gateway_client: GatewayClient,
    good_topology: NymTopology,
    self_address: Recipient,
}

impl PacketSender {
    pub fn new(
        directory_client: Arc<directory_client::Client>,
        good_topology: NymTopology,
        self_address: Recipient,
        gateway_client: GatewayClient,
    ) -> PacketSender {
        PacketSender {
            directory_client,
            gateway_client,
            good_topology,
            self_address,
        }
    }

    pub async fn start_gateway_client(&mut self) {
        self.gateway_client
            .authenticate_and_start()
            .await
            .expect("Couldn't authenticate with gateway node.");
    }
    /// Run some initial checks to ensure our subsequent measurements are valid.
    /// For example, we should be able to send ourselves a Sphinx packet (and receive it
    /// via the websocket, which currently fails.
    pub async fn sanity_check(&mut self) {
        let me = self.self_address.clone();
        let messages = chunker::prepare_messages("hello".to_string(), me, &self.good_topology);
        self.send_messages(messages).await;
    }

    pub async fn send_packets_to_all_nodes(&mut self) {
        let topology = self
            .directory_client
            .get_topology()
            .await
            .expect("couldn't retrieve topology from the directory server");
        for mixnode in topology.mix_nodes {
            self.send_test_packet(mixnode.to_owned()).await;
        }
    }

    async fn send_messages(&mut self, socket_messages: Vec<(NymNodeRoutingAddress, SphinxPacket)>) {
        self.gateway_client
            .batch_send_sphinx_packets(socket_messages)
            .await
            .unwrap();
    }

    async fn send_test_packet(&mut self, mixnode: MixNodePresence) {
        println!("Testing mixnode: {}", mixnode.pub_key);
        let me = self.self_address.clone();
        let topology_to_test = good_topology::new_with_node(mixnode.clone());
        let message = mixnode.pub_key + ":4";
        let messages = chunker::prepare_messages(message, me, &topology_to_test);
        self.send_messages(messages).await;
    }
}
