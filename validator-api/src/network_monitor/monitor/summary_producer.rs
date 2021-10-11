// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::preparer::{InvalidNode, TestedNode};
use crate::network_monitor::test_packet::{NodeType, TestPacket};
use crate::PENALISE_OUTDATED;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct NodeResult {
    pub(crate) identity: String,
    pub(crate) owner: String,
    pub(crate) working: bool,
}

#[derive(Default)]
struct NodeStatus {
    // TODO: will probably be changed to some kind of %
    working: bool,
}

impl NodeStatus {
    fn into_node_status(self, identity: String, owner: String) -> NodeResult {
        NodeResult {
            identity,
            owner,
            working: self.working,
        }
    }
}

#[derive(Default)]
pub(crate) struct TestReport {
    pub(crate) total_sent: usize,
    pub(crate) total_received: usize,
    pub(crate) malformed: Vec<InvalidNode>,

    // again, this will be changed to some % measure when tested multiple times.
    // But tiny steps for now...
    pub(crate) working_mixes: Vec<TestedNode>,
    pub(crate) broken_mixes: Vec<TestedNode>,

    pub(crate) working_gateways: Vec<TestedNode>,
    pub(crate) broken_gateways: Vec<TestedNode>,
}

impl TestReport {
    fn print(&self, detailed: bool) {
        info!("Sent total of {} packets", self.total_sent);
        info!("Received total of {} packets", self.total_received);
        info!("{} nodes are invalid", self.malformed.len());

        info!("{} mixnodes work fine!", self.working_mixes.len());
        info!("{} mixnodes are broken!", self.broken_mixes.len());

        info!("{} gateways work fine!", self.working_gateways.len());
        info!("{} gateways are broken!", self.broken_gateways.len());

        if detailed {
            info!("full summary:");
            for malformed in self.malformed.iter() {
                info!("Malformed: {}", malformed)
            }

            for working in self.working_mixes.iter() {
                info!("Working: {}", working)
            }

            for broken in self.broken_mixes.iter() {
                info!("Broken: {}", broken)
            }

            for working in self.working_gateways.iter() {
                info!("Working: {}", working)
            }

            for broken in self.broken_gateways.iter() {
                info!("Broken: {}", broken)
            }
        }
    }

    fn parse_summary(&mut self, summary: &HashMap<TestedNode, NodeStatus>) {
        for (node, result) in summary.iter() {
            let owned_node = node.clone();
            if node.is_gateway() {
                if result.working {
                    self.working_gateways.push(owned_node)
                } else {
                    self.broken_gateways.push(owned_node)
                }
            } else if result.working {
                self.working_mixes.push(owned_node)
            } else {
                self.broken_mixes.push(owned_node)
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
            let entry = summary.entry(received_status.into()).or_default();
            entry.working = true;
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
