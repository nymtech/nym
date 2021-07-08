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

use crate::network_monitor::monitor::preparer::{PacketPreparer, TestedNode};
use crate::network_monitor::monitor::processor::ReceivedProcessor;
use crate::network_monitor::monitor::sender::PacketSender;
use crate::network_monitor::monitor::summary_producer::{SummaryProducer, TestReport};
use crate::network_monitor::test_packet::NodeType;
use crate::network_monitor::tested_network::TestedNetwork;
use log::{debug, info};
use tokio::time::{sleep, Duration, Instant};

pub(crate) mod preparer;
pub(crate) mod processor;
pub(crate) mod receiver;
pub(crate) mod sender;
pub(crate) mod summary_producer;

const PACKET_DELIVERY_TIMEOUT: Duration = Duration::from_secs(20);
const MONITOR_RUN_INTERVAL: Duration = Duration::from_secs(15 * 60);
const GATEWAY_PING_INTERVAL: Duration = Duration::from_secs(60);

pub(crate) type TodoType = ();

pub(super) struct Monitor {
    nonce: u64,
    packet_preparer: PacketPreparer,
    packet_sender: PacketSender,
    received_processor: ReceivedProcessor,
    summary_producer: SummaryProducer,
    node_status_api_client: TodoType,
    tested_network: TestedNetwork,
}

impl Monitor {
    pub(super) fn new(
        packet_preparer: PacketPreparer,
        packet_sender: PacketSender,
        received_processor: ReceivedProcessor,
        summary_producer: SummaryProducer,
        node_status_api_client: TodoType,
        tested_network: TestedNetwork,
    ) -> Self {
        Monitor {
            nonce: 1,
            packet_preparer,
            packet_sender,
            received_processor,
            summary_producer,
            node_status_api_client,
            tested_network,
        }
    }

    // while it might have been cleaner to put this into a separate `Notifier` structure,
    // I don't see much point considering it's only a single, small, method
    async fn notify_node_status_api(&self, mix_status: TodoType, gateway_status: TodoType) {
        // this will chagne completely and instead will talk to the database
        todo!()
    }

    // checking it this way with a TestReport is rather suboptimal but given the fact we're only
    // doing this fewer than 10 times, it's not that problematic
    fn check_good_nodes_status(&self, report: &TestReport) -> bool {
        for v4_mixes in self.tested_network.v4_topology().mixes().values() {
            for v4_mix in v4_mixes {
                let node = &TestedNode {
                    identity: v4_mix.identity_key.to_base58_string(),
                    owner: v4_mix.owner.clone(),
                    node_type: NodeType::Mixnode,
                };
                if !report.fully_working_mixes.contains(node) {
                    return false;
                }
            }
        }

        for v4_gateway in self.tested_network.v4_topology().gateways() {
            let node = &TestedNode {
                identity: v4_gateway.identity_key.to_base58_string(),
                owner: v4_gateway.owner.clone(),
                node_type: NodeType::Gateway,
            };
            if !report.fully_working_gateways.contains(node) {
                return false;
            }
        }

        for v6_mixes in self.tested_network.v6_topology().mixes().values() {
            for v6_mix in v6_mixes {
                let node = &TestedNode {
                    identity: v6_mix.identity_key.to_base58_string(),
                    owner: v6_mix.owner.clone(),
                    node_type: NodeType::Mixnode,
                };
                if !report.fully_working_mixes.contains(node) {
                    return false;
                }
            }
        }

        for v6_gateway in self.tested_network.v6_topology().gateways() {
            let node = &TestedNode {
                identity: v6_gateway.identity_key.to_base58_string(),
                owner: v6_gateway.owner.clone(),
                node_type: NodeType::Gateway,
            };
            if !report.fully_working_gateways.contains(node) {
                return false;
            }
        }

        true
    }

    async fn test_run(&mut self) {
        info!(target: "Monitor", "Starting test run no. {}", self.nonce);

        debug!(target: "Monitor", "Preparing mix packets to all nodes...");
        let prepared_packets = match self.packet_preparer.prepare_test_packets(self.nonce).await {
            Ok(packets) => packets,
            Err(err) => {
                error!("failed to create packets for the test run - {:?}", err);
                // TODO: return error?
                return;
            }
        };

        self.received_processor.set_new_expected(self.nonce).await;

        info!(target: "Monitor", "Starting to send all the packets...");
        self.packet_sender
            .send_packets(prepared_packets.packets)
            .await;

        info!(
            target: "Monitor",
            "Sending is over, waiting for {:?} before checking what we received",
            PACKET_DELIVERY_TIMEOUT
        );

        // give the packets some time to traverse the network
        sleep(PACKET_DELIVERY_TIMEOUT).await;

        let received = self.received_processor.return_received().await;

        let test_summary = self.summary_producer.produce_summary(
            prepared_packets.tested_nodes,
            received,
            prepared_packets.invalid_nodes,
        );

        // our "good" nodes MUST be working correctly otherwise we cannot trust the results
        if self.check_good_nodes_status(&test_summary.test_report) {
            self.notify_node_status_api(
                test_summary.batch_mix_status,
                test_summary.batch_gateway_status,
            )
            .await;
        } else {
            error!("our own 'good' nodes did not pass the check - we are not going to submit results to the node status API");
        }

        self.nonce += 1;
    }

    async fn ping_all_gateways(&mut self) {
        self.packet_sender.ping_all_active_gateways().await;
    }

    pub(crate) async fn run(&mut self) {
        // start from 0 to run test immediately on startup
        let test_delay = sleep(Duration::from_secs(0));
        tokio::pin!(test_delay);

        let ping_delay = sleep(GATEWAY_PING_INTERVAL);
        tokio::pin!(ping_delay);

        loop {
            tokio::select! {
                _ = &mut test_delay => {
                    self.test_run().await;
                    info!(target: "Monitor", "Next test run will happen in {:?}", MONITOR_RUN_INTERVAL);

                    let now = Instant::now();
                    test_delay.as_mut().reset(now + MONITOR_RUN_INTERVAL);
                    // since we just sent packets through gateways, there's no need to ping them
                    ping_delay.as_mut().reset(now + GATEWAY_PING_INTERVAL);

                }
                _ = &mut ping_delay => {
                    info!(target: "Monitor", "Pinging all active gateways");
                    self.ping_all_gateways().await;

                    let now = Instant::now();
                    ping_delay.as_mut().reset(now + GATEWAY_PING_INTERVAL);
                }
            }
        }
    }
}
