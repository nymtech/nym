// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::preparer::{InvalidNode, TestedNode};
use crate::network_monitor::test_packet::{NodeType, TestPacket};
use crate::PENALISE_OUTDATED;
use std::collections::HashMap;

pub(crate) struct NodeResult {
    pub(crate) pub_key: String,
    pub(crate) owner: String,
    pub(crate) working_ipv4: bool,
    pub(crate) working_ipv6: bool,
}

#[derive(Default)]
struct NodeStatus {
    ip_v4_compatible: bool,
    ip_v6_compatible: bool,
}

impl NodeStatus {
    fn into_node_status(self, pub_key: String, owner: String) -> NodeResult {
        NodeResult {
            pub_key,
            owner,
            working_ipv4: self.ip_v4_compatible,
            working_ipv6: self.ip_v6_compatible,
        }
    }
}

#[derive(Default)]
pub(crate) struct TestReport {
    pub(crate) total_sent: usize,
    pub(crate) total_received: usize,
    pub(crate) malformed: Vec<InvalidNode>,

    // below are only populated if we're going to be printing the report
    pub(crate) only_ipv4_compatible_mixes: Vec<TestedNode>,
    // can't speak v6, but can speak v4
    pub(crate) only_ipv6_compatible_mixes: Vec<TestedNode>,
    // can't speak v4, but can speak v6
    pub(crate) completely_unroutable_mixes: Vec<TestedNode>,
    // can't speak either v4 or v6
    pub(crate) fully_working_mixes: Vec<TestedNode>,

    pub(crate) only_ipv4_compatible_gateways: Vec<TestedNode>,
    // can't speak v6, but can speak v4
    pub(crate) only_ipv6_compatible_gateways: Vec<TestedNode>,
    // can't speak v4, but can speak v6
    pub(crate) completely_unroutable_gateways: Vec<TestedNode>,
    // can't speak either v4 or v6
    pub(crate) fully_working_gateways: Vec<TestedNode>,
}

impl TestReport {
    fn print(&self, detailed: bool) {
        info!("Sent total of {} packets", self.total_sent);
        info!("Received total of {} packets", self.total_received);
        info!("{} nodes are invalid", self.malformed.len());

        info!(
            "{} mixnodes speak ONLY IPv4 (NO IPv6 connectivity)",
            self.only_ipv4_compatible_mixes.len()
        );
        info!(
            "{} mixnodes speak ONLY IPv6 (NO IPv4 connectivity)",
            self.only_ipv6_compatible_mixes.len()
        );
        info!(
            "{} mixnodes are totally unroutable!",
            self.completely_unroutable_mixes.len()
        );
        info!("{} mixnodes work fine!", self.fully_working_mixes.len());

        info!(
            "{} gateways speak ONLY IPv4 (NO IPv6 connectivity)",
            self.only_ipv4_compatible_gateways.len()
        );
        info!(
            "{} gateways speak ONLY IPv6 (NO IPv4 connectivity)",
            self.only_ipv6_compatible_gateways.len()
        );
        info!(
            "{} gateways are totally unroutable!",
            self.completely_unroutable_gateways.len()
        );
        info!("{} gateways work fine!", self.fully_working_gateways.len());

        if detailed {
            info!("full summary:");
            for malformed in self.malformed.iter() {
                info!("{}", malformed)
            }

            for v4_node in self.only_ipv4_compatible_mixes.iter() {
                info!("{}", v4_node)
            }

            for v6_node in self.only_ipv6_compatible_mixes.iter() {
                info!("{}", v6_node)
            }

            for unroutable in self.completely_unroutable_mixes.iter() {
                info!("{}", unroutable)
            }

            for working in self.fully_working_mixes.iter() {
                info!("{}", working)
            }

            for v4_node in self.only_ipv4_compatible_gateways.iter() {
                info!("{}", v4_node)
            }

            for v6_node in self.only_ipv6_compatible_gateways.iter() {
                info!("{}", v6_node)
            }

            for unroutable in self.completely_unroutable_gateways.iter() {
                info!("{}", unroutable)
            }

            for working in self.fully_working_gateways.iter() {
                info!("{}", working)
            }
        }
    }

    fn parse_summary(&mut self, summary: &HashMap<TestedNode, NodeStatus>) {
        for (node, result) in summary.iter() {
            let owned_node = node.clone();
            if node.is_gateway() {
                if result.ip_v4_compatible && result.ip_v6_compatible {
                    self.fully_working_gateways.push(owned_node)
                } else if result.ip_v4_compatible {
                    self.only_ipv4_compatible_gateways.push(owned_node)
                } else if result.ip_v6_compatible {
                    self.only_ipv6_compatible_gateways.push(owned_node)
                } else {
                    self.completely_unroutable_gateways.push(owned_node)
                }
            } else if result.ip_v4_compatible && result.ip_v6_compatible {
                self.fully_working_mixes.push(owned_node)
            } else if result.ip_v4_compatible {
                self.only_ipv4_compatible_mixes.push(owned_node)
            } else if result.ip_v6_compatible {
                self.only_ipv6_compatible_mixes.push(owned_node)
            } else {
                self.completely_unroutable_mixes.push(owned_node)
            }
        }
    }
}

pub(crate) struct TestSummary {
    pub(crate) mixnode_results: Vec<NodeResult>,
    pub(crate) gateway_results: Vec<NodeResult>,
    pub(crate) test_report: TestReport,
}

#[derive(Default)]
pub(crate) struct SummaryProducer {
    print_report: bool,
    print_detailed_report: bool,
}

impl SummaryProducer {
    pub(crate) fn with_report(mut self) -> Self {
        self.print_report = true;
        self
    }

    pub(crate) fn with_detailed_report(mut self) -> Self {
        self.print_report = true;
        self.print_detailed_report = true;
        self
    }

    pub(super) fn produce_summary(
        &self,
        expected_nodes: Vec<TestedNode>,
        received_packets: Vec<TestPacket>,
        invalid_nodes: Vec<InvalidNode>,
    ) -> TestSummary {
        let expected_nodes_count = expected_nodes.len();
        let received_packets_count = received_packets.len();

        // contains map of all (seemingly valid) nodes and whether they speak ipv4/ipv6
        let mut summary: HashMap<TestedNode, NodeStatus> = HashMap::new();

        // update based on data we actually got
        for received_status in received_packets.into_iter() {
            let is_received_v4 = received_status.ip_version().is_v4();
            let entry = summary.entry(received_status.into()).or_default();
            if is_received_v4 {
                entry.ip_v4_compatible = true
            } else {
                entry.ip_v6_compatible = true
            }
        }

        // insert entries we didn't get but were expecting
        for expected in expected_nodes.into_iter() {
            summary.entry(expected).or_default();
        }

        // finally insert malformed nodes
        for malformed in invalid_nodes.iter() {
            match malformed {
                InvalidNode::OutdatedMix(id, owner, _)
                | InvalidNode::OutdatedGateway(id, owner, _) => {
                    if PENALISE_OUTDATED {
                        summary.insert(TestedNode::from_raw_gateway(id, owner), Default::default());
                    }
                }
                InvalidNode::MalformedMix(id, owner) | InvalidNode::MalformedGateway(id, owner) => {
                    summary.insert(TestedNode::from_raw_mix(id, owner), Default::default());
                }
            }
        }

        let mut report = TestReport {
            total_sent: expected_nodes_count * 2, // we sent two packets per node (one ipv4 and one ipv6)
            total_received: received_packets_count,
            malformed: invalid_nodes,

            ..Default::default()
        };

        report.parse_summary(&summary);

        if self.print_report {
            report.print(self.print_detailed_report);
        }

        let (mixes, gateways): (Vec<_>, Vec<_>) = summary
            .into_iter()
            .partition(|(node, _)| node.node_type == NodeType::Mixnode);

        let mixnode_results = mixes
            .into_iter()
            .map(|(node, result)| result.into_node_status(node.identity, node.owner))
            .collect();

        let gateway_results = gateways
            .into_iter()
            .map(|(node, result)| result.into_node_status(node.identity, node.owner))
            .collect();

        TestSummary {
            mixnode_results,
            gateway_results,
            test_report: report,
        }
    }
}
