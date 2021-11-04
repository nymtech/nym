// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::network_monitor::monitor::preparer::{PacketPreparer, TestedNode};
use crate::network_monitor::monitor::processor::ReceivedProcessor;
use crate::network_monitor::monitor::sender::PacketSender;
use crate::network_monitor::monitor::summary_producer::{NodeResult, SummaryProducer, TestReport};
use crate::network_monitor::test_packet::NodeType;
use crate::network_monitor::tested_network::TestedNetwork;
use crate::storage::ValidatorApiStorage;
use log::{debug, error, info, warn};
use std::process;
use tokio::time::{sleep, Duration, Instant};

pub(crate) mod preparer;
pub(crate) mod processor;
pub(crate) mod receiver;
pub(crate) mod sender;
pub(crate) mod summary_producer;

pub(super) struct Monitor {
    nonce: u64,
    packet_preparer: PacketPreparer,
    packet_sender: PacketSender,
    received_processor: ReceivedProcessor,
    summary_producer: SummaryProducer,
    node_status_storage: ValidatorApiStorage,
    tested_network: TestedNetwork,
    run_interval: Duration,
    gateway_ping_interval: Duration,
    packet_delivery_timeout: Duration,
}

impl Monitor {
    pub(super) fn new(
        config: &Config,
        packet_preparer: PacketPreparer,
        packet_sender: PacketSender,
        received_processor: ReceivedProcessor,
        summary_producer: SummaryProducer,
        node_status_storage: ValidatorApiStorage,
        tested_network: TestedNetwork,
    ) -> Self {
        Monitor {
            nonce: 1,
            packet_preparer,
            packet_sender,
            received_processor,
            summary_producer,
            node_status_storage,
            tested_network,
            run_interval: config.get_network_monitor_run_interval(),
            gateway_ping_interval: config.get_gateway_ping_interval(),
            packet_delivery_timeout: config.get_packet_delivery_timeout(),
        }
    }

    // while it might have been cleaner to put this into a separate `Notifier` structure,
    // I don't see much point considering it's only a single, small, method
    async fn submit_new_node_statuses(
        &self,
        mixnode_results: Vec<NodeResult>,
        gateway_results: Vec<NodeResult>,
    ) {
        if let Err(err) = self
            .node_status_storage
            .submit_new_statuses(mixnode_results, gateway_results)
            .await
        {
            // this can only fail if there's an issue with the database - we can't really recover
            error!(
                "Failed to submit new monitoring results to the database - {}",
                err
            );

            // TODO: slightly more graceful shutdown here
            process::exit(1);
        }

        // indicate our run has completed successfully and should be used in any future
        // uptime calculations
        if let Err(err) = self.node_status_storage.insert_monitor_run().await {
            error!(
                "Failed to submit monitor run information to the database - {}",
                err
            );

            // TODO: slightly more graceful shutdown here
            process::exit(1);
        }
    }

    // checking it this way with a TestReport is rather suboptimal but given the fact we're only
    // doing this fewer than 10 times, it's not that problematic
    fn check_good_nodes_status(&self, report: &TestReport) -> bool {
        let mut good_nodes_status = true;
        for v4_mixes in self.tested_network.v4_topology().mixes().values() {
            for v4_mix in v4_mixes {
                let node = &TestedNode {
                    identity: v4_mix.identity_key.to_base58_string(),
                    owner: v4_mix.owner.clone(),
                    node_type: NodeType::Mixnode,
                };
                if !report.fully_working_mixes.contains(node) {
                    warn!("Mixnode {} has not passed the ipv4 check", node.identity);
                    good_nodes_status = false;
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
                warn!("Gateway {} has not passed the ipv4 check", node.identity);
                good_nodes_status = false;
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
                    warn!("Mixnode {} has not passed the ipv6 check", node.identity);
                    good_nodes_status = false;
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
                warn!("Gateway {} has not passed the ipv6 check", node.identity);
                good_nodes_status = false;
            }
        }

        good_nodes_status
    }

    async fn test_run(&mut self) {
        info!(target: "Monitor", "Starting test run no. {}", self.nonce);

        debug!(target: "Monitor", "Preparing mix packets to all nodes...");
        let prepared_packets = self.packet_preparer.prepare_test_packets(self.nonce).await;

        self.received_processor.set_new_expected(self.nonce).await;

        info!(target: "Monitor", "Starting to send all the packets...");
        self.packet_sender
            .send_packets(prepared_packets.packets)
            .await;

        info!(
            target: "Monitor",
            "Sending is over, waiting for {:?} before checking what we received",
            self.packet_delivery_timeout
        );

        // give the packets some time to traverse the network
        sleep(self.packet_delivery_timeout).await;

        let received = self.received_processor.return_received().await;

        let test_summary = self.summary_producer.produce_summary(
            prepared_packets.tested_nodes,
            received,
            prepared_packets.invalid_nodes,
        );

        // our "good" nodes MUST be working correctly otherwise we cannot trust the results
        if self.check_good_nodes_status(&test_summary.test_report) {
            self.submit_new_node_statuses(
                test_summary.mixnode_results,
                test_summary.gateway_results,
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
        self.received_processor.start_receiving();

        // wait for validator cache to be ready
        self.packet_preparer
            .wait_for_validator_cache_initial_values()
            .await;

        // start from 0 to run test immediately on startup
        let test_delay = sleep(Duration::from_secs(0));
        tokio::pin!(test_delay);

        let ping_delay = sleep(self.gateway_ping_interval);
        tokio::pin!(ping_delay);

        loop {
            tokio::select! {
                _ = &mut test_delay => {
                    self.test_run().await;
                    info!(target: "Monitor", "Next test run will happen in {:?}", self.run_interval);

                    let now = Instant::now();
                    test_delay.as_mut().reset(now + self.run_interval);
                    // since we just sent packets through gateways, there's no need to ping them
                    ping_delay.as_mut().reset(now + self.gateway_ping_interval);

                }
                _ = &mut ping_delay => {
                    info!(target: "Monitor", "Pinging all active gateways");
                    self.ping_all_gateways().await;

                    let now = Instant::now();
                    ping_delay.as_mut().reset(now + self.gateway_ping_interval);
                }
            }
        }
    }
}
