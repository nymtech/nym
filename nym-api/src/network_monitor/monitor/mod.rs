// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::monitor::preparer::PacketPreparer;
use crate::network_monitor::monitor::processor::ReceivedProcessor;
use crate::network_monitor::monitor::sender::PacketSender;
use crate::network_monitor::monitor::summary_producer::{SummaryProducer, TestReport, TestSummary};
use crate::network_monitor::test_packet::NodeTestMessage;
use crate::network_monitor::test_route::TestRoute;
use crate::storage::NymApiStorage;
use crate::support::config;
use nym_mixnet_contract_common::NodeId;
use nym_sphinx::params::PacketType;
use nym_sphinx::receiver::MessageReceiver;
use nym_task::TaskClient;
use std::collections::{HashMap, HashSet};
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, error, info, trace};

pub(crate) mod gateway_client_handle;
pub(crate) mod preparer;
pub(crate) mod processor;
pub(crate) mod receiver;
pub(crate) mod sender;
pub(crate) mod summary_producer;

pub(super) struct Monitor<R: MessageReceiver + Send + Sync + 'static> {
    test_nonce: u64,
    packet_preparer: PacketPreparer,
    packet_sender: PacketSender,
    received_processor: ReceivedProcessor<R>,
    summary_producer: SummaryProducer,
    node_status_storage: NymApiStorage,
    run_interval: Duration,
    packet_delivery_timeout: Duration,

    /// Number of test packets sent via each "random" route to verify whether they work correctly.
    route_test_packets: usize,

    /// Desired number of test routes to be constructed (and working) during a monitor test run.
    test_routes: usize,

    /// The minimum number of test routes that need to be constructed (and working) in order for
    /// a monitor test run to be valid.
    minimum_test_routes: usize,

    packet_type: PacketType,
}

impl<R: MessageReceiver + Send + Sync> Monitor<R> {
    pub(super) fn new(
        config: &config::NetworkMonitor,
        packet_preparer: PacketPreparer,
        packet_sender: PacketSender,
        received_processor: ReceivedProcessor<R>,
        summary_producer: SummaryProducer,
        node_status_storage: NymApiStorage,
        packet_type: PacketType,
    ) -> Self {
        Monitor {
            test_nonce: 1,
            packet_preparer,
            packet_sender,
            received_processor,
            summary_producer,
            node_status_storage,
            run_interval: config.debug.run_interval,
            packet_delivery_timeout: config.debug.packet_delivery_timeout,
            route_test_packets: config.debug.route_test_packets,
            test_routes: config.debug.test_routes,
            minimum_test_routes: config.debug.minimum_test_routes,
            packet_type,
        }
    }

    // while it might have been cleaner to put this into a separate `Notifier` structure,
    // I don't see much point considering it's only a single, small, method
    async fn submit_new_node_statuses(&mut self, test_summary: TestSummary, report: TestReport) {
        // indicate our run has completed successfully and should be used in any future
        // uptime calculations
        let monitor_run_id = match self
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
            Ok(id) => id,
            Err(err) => {
                error!("Failed to submit monitor run information to the database: {err}",);
                return;
            }
        };

        if let Err(err) = self
            .node_status_storage
            .insert_monitor_run_report(report, monitor_run_id)
            .await
        {
            error!("failed to submit monitor run report to the database: {err}",);
        }

        info!("finished persisting monitor run with id {monitor_run_id}");
    }

    fn analyse_received_test_route_packets(
        &self,
        packets: &[NodeTestMessage],
    ) -> HashMap<u64, usize> {
        let mut received = HashMap::new();
        for packet in packets {
            *received.entry(packet.ext.route_id).or_insert(0usize) += 1usize
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
            let mut packet_preparer = self.packet_preparer.clone();
            let route = route.clone();
            let gateway_packets = packet_preparer.prepare_test_route_viability_packets(
                &route,
                self.route_test_packets,
                self.packet_type,
            );
            packets.push(gateway_packets);
        }

        self.received_processor.set_route_test_nonce();
        let gateway_clients = self.packet_sender.send_packets(packets).await;

        // give the packets some time to traverse the network
        sleep(self.packet_delivery_timeout).await;

        // start all the disconnections in the background
        drop(gateway_clients);

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

    fn blacklist_route_nodes(&self, route: &TestRoute, blacklist: &mut HashSet<NodeId>) {
        for mix in route.topology().mixnodes() {
            blacklist.insert(mix.node_id);
        }
        blacklist.insert(route.gateway().node_id);
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
                .prepare_test_routes(remaining * 2)
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
                // SAFETY: the results is subset of candidates so the entry must exist
                #[allow(clippy::unwrap_used)]
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
            .prepare_test_packets(self.test_nonce, routes, self.packet_type)
            .await;

        let total_sent = prepared_packets
            .packets
            .iter()
            .flat_map(|packets| packets.packets.iter())
            .count();

        self.received_processor.set_new_test_nonce(self.test_nonce);

        info!("Sending packets to all gateways...");
        let gateway_clients = self
            .packet_sender
            .send_packets(prepared_packets.packets)
            .await;

        info!(
            "Sending is over, waiting for {:?} before checking what we received",
            self.packet_delivery_timeout
        );

        // give the packets some time to traverse the network
        sleep(self.packet_delivery_timeout).await;

        // start all the disconnections in the background
        drop(gateway_clients);

        let received = self.received_processor.return_received().await;
        let total_received = received.len();
        info!("Test routes: {:#?}", routes);
        info!("Received {}/{} packets", total_received, total_sent);

        let summary = self.summary_producer.produce_summary(
            prepared_packets.mixnodes_under_test,
            prepared_packets.gateways_under_test,
            received,
            prepared_packets.invalid_mixnodes,
            prepared_packets.invalid_gateways,
            routes,
        );

        let report = summary.create_report(total_sent, total_received);

        let display_report = summary
            .create_report(total_sent, total_received)
            .to_display_report(&summary.route_results);
        info!("{display_report}");

        self.submit_new_node_statuses(summary, report).await;
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

    pub(crate) async fn run(&mut self, mut shutdown: TaskClient) {
        self.received_processor.start_receiving();

        // wait for validator cache to be ready
        self.packet_preparer
            .wait_for_validator_cache_initial_values(self.minimum_test_routes)
            .await;

        let mut run_interval = tokio::time::interval(self.run_interval);
        while !shutdown.is_shutdown() {
            tokio::select! {
                _  = run_interval.tick() => {
                    tokio::select! {
                        biased;
                        _ = shutdown.recv() => {
                            trace!("UpdateHandler: Received shutdown");
                        }
                        _ = self.test_run() => (),
                    }
                }
                _ = shutdown.recv() => {
                    trace!("UpdateHandler: Received shutdown");
                }
            }
        }
    }
}
