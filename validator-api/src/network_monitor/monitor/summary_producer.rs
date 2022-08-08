// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::preparer::{InvalidNode, TestedNode};
use crate::network_monitor::test_packet::{NodeType, TestPacket};
use crate::network_monitor::test_route::TestRoute;
use mixnet_contract_common::NodeId;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

const INVALID_MIX_ID: u32 = u32::MAX;

// just some approximate measures to print to stdout (well, technically stderr since it's being printed via log)
const EXCEPTIONAL_THRESHOLD: u8 = 95; // 95 - 100
const FINE_THRESHOLD: u8 = 80; // 80 - 95
const POOR_THRESHOLD: u8 = 60; // 60 - 80
const UNRELIABLE_THRESHOLD: u8 = 1; // 1 - 60

// I didn't have time to implement it for this PR, however, an idea for the future is as follows:
// After testing network against N routes, if any one of them is worse than ALLOWED_RELIABILITY_DEVIATION
// from the average result, remove this data and recalculate scores.
// const ALLOWED_RELIABILITY_DEVIATION: f32 = 5.0;

#[derive(Debug)]
pub(crate) struct MixnodeResult {
    pub(crate) mix_id: NodeId,
    pub(crate) identity: String,
    pub(crate) owner: String,
    pub(crate) reliability: u8,
}

impl MixnodeResult {
    pub(crate) fn new(mix_id: NodeId, identity: String, owner: String, reliability: u8) -> Self {
        MixnodeResult {
            mix_id,
            identity,
            owner,
            reliability,
        }
    }
}

#[derive(Debug)]
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
    reliability: u8,
}

impl RouteResult {
    pub(crate) fn new(route: TestRoute, reliability: u8) -> Self {
        RouteResult { route, reliability }
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
                route_result.route, route_result.reliability
            )?;
        }

        writeln!(
            f,
            "Exceptional mixnodes (reliability >= {}): {}",
            EXCEPTIONAL_THRESHOLD, self.exceptional_mixnodes
        )?;
        writeln!(
            f,
            "Exceptional gateways (reliability >= {}): {}",
            EXCEPTIONAL_THRESHOLD, self.exceptional_gateways
        )?;

        writeln!(
            f,
            "Fine mixnodes (reliability {} - {}): {}",
            FINE_THRESHOLD, EXCEPTIONAL_THRESHOLD, self.fine_mixnodes
        )?;
        writeln!(
            f,
            "Fine gateways (reliability {} - {}): {}",
            FINE_THRESHOLD, EXCEPTIONAL_THRESHOLD, self.fine_gateways
        )?;

        writeln!(
            f,
            "Poor mixnodes (reliability {} - {}): {}",
            POOR_THRESHOLD, FINE_THRESHOLD, self.poor_mixnodes
        )?;
        writeln!(
            f,
            "Poor gateways (reliability {} - {}): {}",
            POOR_THRESHOLD, FINE_THRESHOLD, self.poor_gateways
        )?;

        writeln!(
            f,
            "Unreliable mixnodes (reliability {} - {}): {}",
            UNRELIABLE_THRESHOLD, POOR_THRESHOLD, self.unreliable_mixnodes
        )?;
        writeln!(
            f,
            "Unreliable gateways (reliability {} - {}): {}",
            UNRELIABLE_THRESHOLD, POOR_THRESHOLD, self.unreliable_gateways
        )?;

        writeln!(
            f,
            "Unroutable mixnodes (reliability < {}): {}",
            UNRELIABLE_THRESHOLD, self.unroutable_mixnodes
        )?;
        writeln!(
            f,
            "Unroutable gateways (reliability < {}): {}",
            UNRELIABLE_THRESHOLD, self.unroutable_gateways
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
        tested_mixnodes: Vec<TestedNode>,
        tested_gateways: Vec<TestedNode>,
        received_packets: Vec<TestPacket>,
        invalid_mixnodes: Vec<InvalidNode>,
        invalid_gateways: Vec<InvalidNode>,
        test_routes: &[TestRoute],
    ) -> TestSummary {
        let mut raw_mixnode_results = HashMap::new();
        let mut raw_gateway_results = HashMap::new();

        let mut raw_route_results = HashMap::new();

        // we expect each route to receive this many packets in the ideal world
        let per_route_expected =
            (tested_mixnodes.len() + tested_gateways.len()) * self.per_node_test_packets;
        let per_node_expected = test_routes.len() * self.per_node_test_packets;

        // TODO: whenever somebody feels like it, this should really get refactored.
        // probably some wrapper struct would be appropriate here.
        for tested_mixnode in tested_mixnodes {
            raw_mixnode_results.insert(
                (
                    tested_mixnode.mix_id().unwrap_or(INVALID_MIX_ID),
                    tested_mixnode.identity,
                    tested_mixnode.owner,
                ),
                0,
            );
        }

        for tested_gateway in tested_gateways {
            raw_gateway_results.insert((tested_gateway.identity, tested_gateway.owner), 0);
        }

        for invalid_mixnode in invalid_mixnodes {
            raw_mixnode_results.insert(
                (
                    invalid_mixnode.mix_id().unwrap_or(INVALID_MIX_ID),
                    invalid_mixnode.identity(),
                    invalid_mixnode.owner(),
                ),
                0,
            );
        }

        for invalid_gateway in invalid_gateways {
            raw_gateway_results.insert((invalid_gateway.identity(), invalid_gateway.owner()), 0);
        }

        for test_route in test_routes {
            raw_route_results.insert(test_route.id(), 0);
        }

        for received in received_packets {
            let pub_key = received.pub_key.to_base58_string();

            match received.node_type {
                NodeType::Mixnode(mix_id) => {
                    *raw_mixnode_results
                        .entry((mix_id, pub_key, received.owner))
                        .or_default() += 1usize;
                }
                NodeType::Gateway => {
                    *raw_gateway_results
                        .entry((pub_key, received.owner))
                        .or_default() += 1usize;
                }
            }

            *raw_route_results.entry(received.route_id).or_default() += 1usize;
        }

        let mixnode_results = raw_mixnode_results
            .into_iter()
            .map(|((mix_id, identity_key, owner), received)| {
                let reliability =
                    (received as f32 / per_node_expected as f32 * 100.0).round() as u8;
                MixnodeResult::new(mix_id, identity_key, owner, reliability)
            })
            .collect();

        let gateway_results = raw_gateway_results
            .into_iter()
            .map(|((identity_key, owner), received)| {
                let reliability =
                    (received as f32 / per_node_expected as f32 * 100.0).round() as u8;
                GatewayResult::new(identity_key, owner, reliability)
            })
            .collect();

        let route_results = raw_route_results
            .into_iter()
            .filter_map(|(id, received)| {
                let reliability =
                    (received as f32 / per_route_expected as f32 * 100.0).round() as u8;

                // this might be suboptimal as we're going through the entire slice every time
                // but realistically this slice will never have more than ~ 10 elements AT MOST
                test_routes
                    .iter()
                    .find(|route| route.id() == id)
                    .map(|route| RouteResult::new(route.clone(), reliability))
            })
            .collect();

        TestSummary {
            mixnode_results,
            gateway_results,
            route_results,
        }
    }
}
