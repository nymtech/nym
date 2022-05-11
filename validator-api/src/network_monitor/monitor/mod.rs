// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::network_monitor::monitor::preparer::PacketPreparer;
use crate::network_monitor::monitor::processor::ReceivedProcessor;
use crate::network_monitor::monitor::sender::PacketSender;
use crate::network_monitor::monitor::summary_producer::{SummaryProducer, TestSummary};
use crate::network_monitor::test_packet::TestPacket;
use crate::network_monitor::test_route::TestRoute;
use crate::storage::ValidatorApiStorage;
use log::{debug, error, info};
use std::collections::{HashMap, HashSet};
use std::process;
use tokio::time::{sleep, Duration, Instant};

pub(crate) mod gateway_clients_cache;
pub(crate) mod gateways_pinger;
pub(crate) mod preparer;
pub(crate) mod processor;
pub(crate) mod receiver;
pub(crate) mod sender;
pub(crate) mod summary_producer;

pub(super) struct Monitor {
    test_nonce: u64,
    packet_preparer: PacketPreparer,
    packet_sender: PacketSender,
    received_processor: ReceivedProcessor,
    summary_producer: SummaryProducer,
    node_status_storage: ValidatorApiStorage,
    run_interval: Duration,
    gateway_ping_interval: Duration,
    packet_delivery_timeout: Duration,

    /// Number of test packets sent via each "random" route to verify whether they work correctly.
    route_test_packets: usize,

    /// Desired number of test routes to be constructed (and working) during a monitor test run.
    test_routes: usize,

    /// The minimum number of test routes that need to be constructed (and working) in order for
    /// a monitor test run to be valid.
    minimum_test_routes: usize,
}

impl Monitor {
    pub(super) fn new(
        config: &Config,
        packet_preparer: PacketPreparer,
        packet_sender: PacketSender,
        received_processor: ReceivedProcessor,
        summary_producer: SummaryProducer,
        node_status_storage: ValidatorApiStorage,
    ) -> Self {
        Monitor {
            test_nonce: 1,
            packet_preparer,
            packet_sender,
            received_processor,
            summary_producer,
            node_status_storage,
            run_interval: config.get_network_monitor_run_interval(),
            gateway_ping_interval: config.get_gateway_ping_interval(),
            packet_delivery_timeout: config.get_packet_delivery_timeout(),
            route_test_packets: config.get_route_test_packets(),
            test_routes: config.get_test_routes(),
            minimum_test_routes: config.get_minimum_test_routes(),
        }
    }

    // while it might have been cleaner to put this into a separate `Notifier` structure,
    // I don't see much point considering it's only a single, small, method
    async fn submit_new_node_statuses(&mut self, test_summary: TestSummary) {
        // indicate our run has completed successfully and should be used in any future
        // uptime calculations
        if let Err(err) = self
            .node_status_storage
            .insert_monitor_run_results(
                test_summary.mixnode_results,
                test_summary.gateway_results,
                test_summary
                    .route_results
                    .into_iter()
                    .map(|result| result.route)
                    .collect(),
            )
            .await
        {
            error!(
                "Failed to submit monitor run information to the database - {}",
                err
            );

            // TODO: slightly more graceful shutdown here
            process::exit(1);
        }
    }

    fn analyse_received_test_route_packets(&self, packets: &[TestPacket]) -> HashMap<u64, usize> {
        let mut received = HashMap::new();
        for packet in packets {
            *received.entry(packet.route_id).or_insert(0usize) += 1usize
        }

        received
    }

    async fn test_chosen_test_routes(&mut self, routes: &[TestRoute]) -> HashMap<u64, bool> {
        // notes for the future improvements:
        /*
           - gateway authentication failure should only 'blacklist' gateways, not mixnodes

        */

        if routes.is_empty() {
            return HashMap::new();
        }

        debug!("Testing the following test routes: {:#?}", routes);

        let mut packets = Vec::with_capacity(routes.len());
        for route in routes {
            packets.push(
                self.packet_preparer
                    .prepare_test_route_viability_packets(route, self.route_test_packets)
                    .await,
            );
        }

        self.received_processor.set_route_test_nonce().await;
        self.packet_sender.send_packets(packets).await;

        // give the packets some time to traverse the network
        sleep(self.packet_delivery_timeout).await;

        let received = self.received_processor.return_received().await;
        let mut results = self.analyse_received_test_route_packets(&received);

        // create entry for routes that might have not forwarded a single packet
        for route in routes {
            results.entry(route.id()).or_insert(0);
        }

        for entry in results.iter() {
            if *entry.1 == self.route_test_packets {
                info!("✔️ {} succeeded", entry.0)
            } else {
                info!(
                    "❌️ {} failed ({}/{} received)",
                    entry.0, entry.1, self.route_test_packets
                )
            }
        }

        results
            .into_iter()
            .map(|(k, v)| (k, v == self.route_test_packets))
            .collect()
    }

    fn blacklist_route_nodes(&self, route: &TestRoute, blacklist: &mut HashSet<String>) {
        for mix in route.topology().mixes_as_vec() {
            blacklist.insert(mix.identity_key.to_base58_string());
        }
        blacklist.insert(route.gateway_identity().to_base58_string());
    }

    async fn prepare_test_routes(&mut self) -> Option<Vec<TestRoute>> {
        info!("Generating test routes...");

        // keep track of nodes that should not be used for route construction
        let mut blacklist = HashSet::new();
        let mut verified_routes = Vec::new();
        let mut remaining = self.test_routes;
        let mut current_attempt = 0;

        // todo: tweak this to something more appropriate
        let max_attempts = self.test_routes * 10;

        'outer: loop {
            if current_attempt >= max_attempts {
                if verified_routes.len() >= self.minimum_test_routes {
                    return Some(verified_routes);
                }
                return None;
            }

            // try to construct slightly more than what we actually need to more quickly reach
            // the actual target
            let candidates = match self
                .packet_preparer
                .prepare_test_routes(remaining * 2, &mut blacklist)
                .await
            {
                Some(candidates) => candidates,
                // if there are no more routes to generate, see if we have managed to construct
                // at least the minimum number of routes
                None => {
                    if verified_routes.len() >= self.minimum_test_routes {
                        return Some(verified_routes);
                    }
                    return None;
                }
            };
            let results = self.test_chosen_test_routes(&candidates).await;
            for candidate in candidates {
                // ideally we would blacklist all nodes regardless of the result so we would not use them anymore
                // however, currently we have huge imbalance of gateways to mixnodes so we might accidentally
                // discard working gateway because it was paired with broken mixnode
                if *results.get(&candidate.id()).unwrap() {
                    // if the path is fully working, blacklist those nodes so we wouldn't construct
                    // any other path through any of those nodes
                    self.blacklist_route_nodes(&candidate, &mut blacklist);

                    verified_routes.push(candidate);
                    if verified_routes.len() == self.test_routes {
                        break 'outer;
                    }
                }
            }

            remaining = self.test_routes - verified_routes.len();
            current_attempt += 1;
        }

        Some(verified_routes)
    }

    async fn test_network_against(&mut self, routes: &[TestRoute]) {
        info!("Generating test mix packets for all the network nodes...");
        let prepared_packets = self
            .packet_preparer
            .prepare_test_packets(self.test_nonce, routes)
            .await;

        let total_sent = prepared_packets
            .packets
            .iter()
            .flat_map(|packets| packets.packets.iter())
            .count();

        self.received_processor
            .set_new_test_nonce(self.test_nonce)
            .await;

        info!("Sending packets to all gateways...");
        self.packet_sender
            .send_packets(prepared_packets.packets)
            .await;

        info!(
            "Sending is over, waiting for {:?} before checking what we received",
            self.packet_delivery_timeout
        );

        // give the packets some time to traverse the network
        sleep(self.packet_delivery_timeout).await;

        let received = self.received_processor.return_received().await;
        let total_received = received.len();
        info!("Test routes: {:?}", routes);
        info!("Received {}/{} packets", total_received, total_sent);

        let summary = self.summary_producer.produce_summary(
            prepared_packets.tested_mixnodes,
            prepared_packets.tested_gateways,
            received,
            prepared_packets.invalid_mixnodes,
            prepared_packets.invalid_gateways,
            routes,
        );

        let report = summary.create_report(total_sent, total_received);
        info!("{}", report);

        self.submit_new_node_statuses(summary).await;
    }

    async fn test_run(&mut self) {
        info!("Starting test run no. {}", self.test_nonce);
        let start = Instant::now();

        if let Some(test_routes) = self.prepare_test_routes().await {
            info!(
                "Determined reliable routes to test all other nodes against. : {:?}",
                test_routes
            );
            self.test_network_against(&test_routes).await;
        } else {
            error!("We failed to construct sufficient number of test routes to test the network against")
        }

        debug!("Test run took {:?}", Instant::now().duration_since(start));

        self.test_nonce += 1;
    }

    pub(crate) async fn run(&mut self) {
        self.received_processor.start_receiving();

        // wait for validator cache to be ready
        self.packet_preparer
            .wait_for_validator_cache_initial_values(self.minimum_test_routes)
            .await;

        self.packet_sender
            .spawn_gateways_pinger(self.gateway_ping_interval);

        let mut run_interval = tokio::time::interval(self.run_interval);
        loop {
            run_interval.tick().await;
            self.test_run().await;
        }
    }
}
