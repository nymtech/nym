// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::monitor::preparer::InvalidNode;
use crate::network_monitor::test_packet::NodeTestMessage;
use crate::network_monitor::test_route::TestRoute;
use nym_mixnet_contract_common::MixId;
use nym_node_tester_utils::node::{NodeType, TestableNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct MixnodeResult {
    pub(crate) mix_id: MixId,
    pub(crate) identity: String,
    pub(crate) owner: String,
    pub(crate) reliability: u8,
}

impl MixnodeResult {
    pub(crate) fn new(mix_id: MixId, identity: String, owner: String, reliability: u8) -> Self {
        MixnodeResult {
            mix_id,
            identity,
            owner,
            reliability,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub(crate) struct GatewayResult {
    pub(crate) identity: String,
    pub(crate) owner: String,
    pub(crate) reliability: u8,
}

impl GatewayResult {
    pub(crate) fn new(identity: String, owner: String, reliability: u8) -> Self {
        GatewayResult {
            identity,
            owner,
            reliability,
        }
    }
}

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

#[derive(Default, Debug)]
pub(crate) struct TestReport {
    pub(crate) network_reliability: f32,
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

impl TestReport {
    fn new(
        total_sent: usize,
        total_received: usize,
        mixnode_results: &[MixnodeResult],
        gateway_results: &[GatewayResult],
        route_results: &[RouteResult],
    ) -> Self {
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

        for mixnode_result in mixnode_results {
            if mixnode_result.reliability >= EXCEPTIONAL_THRESHOLD {
                exceptional_mixnodes += 1;
            } else if mixnode_result.reliability >= FINE_THRESHOLD {
                fine_mixnodes += 1;
            } else if mixnode_result.reliability >= POOR_THRESHOLD {
                poor_mixnodes += 1;
            } else if mixnode_result.reliability >= UNRELIABLE_THRESHOLD {
                unreliable_mixnodes += 1;
            } else {
                unroutable_mixnodes += 1;
            }
        }

        for gateway_result in gateway_results {
            if gateway_result.reliability >= EXCEPTIONAL_THRESHOLD {
                exceptional_gateways += 1;
            } else if gateway_result.reliability >= FINE_THRESHOLD {
                fine_gateways += 1;
            } else if gateway_result.reliability >= POOR_THRESHOLD {
                poor_gateways += 1;
            } else if gateway_result.reliability >= UNRELIABLE_THRESHOLD {
                unreliable_gateways += 1;
            } else {
                unroutable_gateways += 1;
            }
        }

        let network_reliability = total_received as f32 / total_sent as f32 * 100.0;

        TestReport {
            network_reliability,
            total_sent,
            total_received,
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

impl Display for TestReport {
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
    pub(crate) mixnode_results: Vec<MixnodeResult>,
    pub(crate) gateway_results: Vec<GatewayResult>,
    pub(crate) route_results: Vec<RouteResult>,
}

impl TestSummary {
    pub(crate) fn create_report(&self, total_sent: usize, total_received: usize) -> TestReport {
        TestReport::new(
            total_sent,
            total_received,
            &self.mixnode_results,
            &self.gateway_results,
            &self.route_results,
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

            match node.typ {
                NodeType::Mixnode { mix_id } => {
                    let res =
                        MixnodeResult::new(mix_id, node.encoded_identity, node.owner, reliability);
                    mixnode_results.push(res)
                }
                NodeType::Gateway => {
                    let res = GatewayResult::new(node.encoded_identity, node.owner, reliability);
                    gateway_results.push(res)
                }
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
