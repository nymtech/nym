// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::monitor::preparer::InvalidNode;
use crate::network_monitor::test_packet::NodeTestMessage;
use crate::network_monitor::test_route::TestRoute;
use nym_node_tester_utils::node::{NodeType, TestableNode};
use nym_types::monitoring::NodeResult;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

// just some approximate measures to print to stdout (well, technically stderr since it's being printed via log)
const EXCEPTIONAL_THRESHOLD: u8 = 95; // 95 - 100
const FINE_THRESHOLD: u8 = 80; // 80 - 95
const POOR_THRESHOLD: u8 = 60; // 60 - 80
const UNRELIABLE_THRESHOLD: u8 = 1; // 1 - 60

// I didn't have time to implement it for this PR, however, an idea for the future is as follows:
// After testing network against N routes, if any one of them is worse than ALLOWED_RELIABILITY_DEVIATION
// from the average result, remove this data and recalculate scores.
// const ALLOWED_RELIABILITY_DEVIATION: f32 = 5.0;

#[derive(Debug, Clone)]
pub(crate) struct RouteResult {
    pub(crate) route: TestRoute,
    performance: f32,
}

impl RouteResult {
    pub(crate) fn new(route: TestRoute, performance: f32) -> Self {
        RouteResult { route, performance }
    }
}

#[derive(Debug)]
pub(crate) struct TestReport {
    pub(crate) network_reliability: f64,
    pub(crate) total_sent: usize,
    pub(crate) total_received: usize,

    // integer score to number of nodes with that score
    pub(crate) mixnode_results: HashMap<u8, usize>,
    pub(crate) gateway_results: HashMap<u8, usize>,
}

impl TestReport {
    pub(crate) fn new(
        total_sent: usize,
        total_received: usize,
        raw_mixnode_results: &[NodeResult],
        raw_gateway_results: &[NodeResult],
    ) -> Self {
        let network_reliability = total_received as f64 / total_sent as f64 * 100.0;

        let mut mixnode_results = HashMap::new();
        let mut gateway_results = HashMap::new();

        for res in raw_mixnode_results {
            mixnode_results
                .entry(res.reliability)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }

        for res in raw_gateway_results {
            gateway_results
                .entry(res.reliability)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }

        TestReport {
            network_reliability,
            total_sent,
            total_received,
            mixnode_results,
            gateway_results,
        }
    }

    pub(crate) fn to_display_report(&self, route_results: &[RouteResult]) -> DisplayTestReport {
        let mut exceptional_mixnodes = 0;
        let mut exceptional_gateways = 0;

        let mut fine_mixnodes = 0;
        let mut fine_gateways = 0;

        let mut poor_mixnodes = 0;
        let mut poor_gateways = 0;

        let mut unreliable_mixnodes = 0;
        let mut unreliable_gateways = 0;

        let mut unroutable_mixnodes = 0;
        let mut unroutable_gateways = 0;

        for (&score, &count) in &self.mixnode_results {
            if score >= EXCEPTIONAL_THRESHOLD {
                exceptional_mixnodes += count;
            } else if score >= FINE_THRESHOLD {
                fine_mixnodes += count;
            } else if score >= POOR_THRESHOLD {
                poor_mixnodes += count;
            } else if score >= UNRELIABLE_THRESHOLD {
                unreliable_mixnodes += count;
            } else {
                unroutable_mixnodes += count;
            }
        }

        for (&score, &count) in &self.gateway_results {
            if score >= EXCEPTIONAL_THRESHOLD {
                exceptional_gateways += count;
            } else if score >= FINE_THRESHOLD {
                fine_gateways += count;
            } else if score >= POOR_THRESHOLD {
                poor_gateways += count;
            } else if score >= UNRELIABLE_THRESHOLD {
                unreliable_gateways += count;
            } else {
                unroutable_gateways += count;
            }
        }

        DisplayTestReport {
            network_reliability: self.network_reliability,
            total_sent: self.total_sent,
            total_received: self.total_received,
            route_results: route_results.to_vec(),
            exceptional_mixnodes,
            exceptional_gateways,
            fine_mixnodes,
            fine_gateways,
            poor_mixnodes,
            poor_gateways,
            unreliable_mixnodes,
            unreliable_gateways,
            unroutable_mixnodes,
            unroutable_gateways,
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct DisplayTestReport {
    pub(crate) network_reliability: f64,
    pub(crate) total_sent: usize,
    pub(crate) total_received: usize,

    pub(crate) route_results: Vec<RouteResult>,

    pub(crate) exceptional_mixnodes: usize,
    pub(crate) exceptional_gateways: usize,

    pub(crate) fine_mixnodes: usize,
    pub(crate) fine_gateways: usize,

    pub(crate) poor_mixnodes: usize,
    pub(crate) poor_gateways: usize,

    pub(crate) unreliable_mixnodes: usize,
    pub(crate) unreliable_gateways: usize,

    pub(crate) unroutable_mixnodes: usize,
    pub(crate) unroutable_gateways: usize,
}

impl Display for DisplayTestReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Mix Network Test Report")?;
        writeln!(
            f,
            "Overall reliability: {:.2} ({} / {} received)",
            self.network_reliability, self.total_received, self.total_sent,
        )?;

        writeln!(f, "Routes used for testing:")?;
        for route_result in &self.route_results {
            writeln!(
                f,
                "{:?}, reliability: {:.2}",
                route_result.route, route_result.performance
            )?;
        }

        writeln!(
            f,
            "Exceptional mixnodes (reliability >= {EXCEPTIONAL_THRESHOLD}): {}",
            self.exceptional_mixnodes
        )?;
        writeln!(
            f,
            "Exceptional gateways (reliability >= {EXCEPTIONAL_THRESHOLD}): {}",
            self.exceptional_gateways
        )?;

        writeln!(
            f,
            "Fine mixnodes (reliability {FINE_THRESHOLD} - {EXCEPTIONAL_THRESHOLD}): {}",
            self.fine_mixnodes
        )?;
        writeln!(
            f,
            "Fine gateways (reliability {FINE_THRESHOLD} - {EXCEPTIONAL_THRESHOLD}): {}",
            self.fine_gateways
        )?;

        writeln!(
            f,
            "Poor mixnodes (reliability {POOR_THRESHOLD} - {FINE_THRESHOLD}): {}",
            self.poor_mixnodes
        )?;
        writeln!(
            f,
            "Poor gateways (reliability {POOR_THRESHOLD} - {FINE_THRESHOLD}): {}",
            self.poor_gateways
        )?;

        writeln!(
            f,
            "Unreliable mixnodes (reliability {UNRELIABLE_THRESHOLD} - {POOR_THRESHOLD}): {}",
            self.unreliable_mixnodes
        )?;
        writeln!(
            f,
            "Unreliable gateways (reliability {UNRELIABLE_THRESHOLD} - {POOR_THRESHOLD}): {}",
            self.unreliable_gateways
        )?;

        writeln!(
            f,
            "Unroutable mixnodes (reliability < {UNRELIABLE_THRESHOLD}): {}",
            self.unroutable_mixnodes
        )?;
        writeln!(
            f,
            "Unroutable gateways (reliability < {UNRELIABLE_THRESHOLD}): {}",
            self.unroutable_gateways
        )?;

        Ok(())
    }
}

pub(crate) struct TestSummary {
    pub(crate) mixnode_results: Vec<NodeResult>,
    pub(crate) gateway_results: Vec<NodeResult>,
    pub(crate) route_results: Vec<RouteResult>,
}

impl TestSummary {
    pub(crate) fn create_report(&self, total_sent: usize, total_received: usize) -> TestReport {
        TestReport::new(
            total_sent,
            total_received,
            &self.mixnode_results,
            &self.gateway_results,
        )
    }
}

#[derive(Default)]
pub(crate) struct SummaryProducer {
    per_node_test_packets: usize,
    print_report: bool,
}

impl SummaryProducer {
    pub(crate) fn new(per_node_test_packets: usize) -> Self {
        SummaryProducer {
            per_node_test_packets,
            print_report: false,
        }
    }

    pub(crate) fn with_report(mut self) -> Self {
        self.print_report = true;
        self
    }

    pub(super) fn produce_summary(
        &self,
        tested_mixnodes: Vec<TestableNode>,
        tested_gateways: Vec<TestableNode>,
        received_packets: Vec<NodeTestMessage>,
        invalid_mixnodes: Vec<InvalidNode>,
        invalid_gateways: Vec<InvalidNode>,
        test_routes: &[TestRoute],
    ) -> TestSummary {
        // we expect each route to receive this many packets in the ideal world
        let per_route_expected =
            (tested_mixnodes.len() + tested_gateways.len()) * self.per_node_test_packets;
        let per_node_expected = test_routes.len() * self.per_node_test_packets;

        let mut raw_route_results = HashMap::new();
        for test_route in test_routes {
            raw_route_results.insert(test_route.id(), 0);
        }

        let mut raw_results = HashMap::new();

        for tested_mixnode in tested_mixnodes {
            raw_results.insert(tested_mixnode, 0);
        }

        for tested_gateway in tested_gateways {
            raw_results.insert(tested_gateway, 0);
        }

        for invalid_mixnode in invalid_mixnodes {
            raw_results.insert(invalid_mixnode.into(), 0);
        }

        for invalid_gateway in invalid_gateways {
            raw_results.insert(invalid_gateway.into(), 0);
        }

        for received in received_packets {
            *raw_results.entry(received.tested_node).or_default() += 1usize;
            *raw_route_results.entry(received.ext.route_id).or_default() += 1usize;
        }

        let mut mixnode_results = Vec::new();
        let mut gateway_results = Vec::new();

        for (node, received) in raw_results {
            let performance = received as f32 / per_node_expected as f32 * 100.0;
            let reliability = performance.round() as u8;

            let result = NodeResult::new(node.node_id, node.encoded_identity, reliability);
            match node.typ {
                NodeType::Mixnode => mixnode_results.push(result),
                NodeType::Gateway => gateway_results.push(result),
            }
        }

        let route_results = raw_route_results
            .into_iter()
            .filter_map(|(id, received)| {
                let performance = received as f32 / per_route_expected as f32 * 100.0;

                // this might be suboptimal as we're going through the entire slice every time
                // but realistically this slice will never have more than ~ 10 elements AT MOST
                test_routes
                    .iter()
                    .find(|route| route.id() == id)
                    .map(|route| RouteResult::new(route.clone(), performance))
            })
            .collect();

        TestSummary {
            mixnode_results,
            gateway_results,
            route_results,
        }
    }
}
