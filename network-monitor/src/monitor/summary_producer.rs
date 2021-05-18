// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::monitor::preparer::{InvalidNode, TestedNode};
use crate::node_status_api::models::{
    BatchGatewayStatus, BatchMixStatus, GatewayStatus, MixStatus,
};
use crate::test_packet::{NodeType, TestPacket};
use crate::PENALISE_OUTDATED;
use log::*;
use std::collections::HashMap;

#[derive(Default)]
struct NodeResult {
    ip_v4_compatible: bool,
    ip_v6_compatible: bool,
}

impl NodeResult {
    fn into_mix_status(self, pub_key: String, owner: String) -> Vec<MixStatus> {
        let v4_status = MixStatus {
            owner: owner.clone(),
            pub_key: pub_key.clone(),
            ip_version: "4".to_string(),
            up: self.ip_v4_compatible,
        };

        let v6_status = MixStatus {
            owner,
            pub_key,
            ip_version: "6".to_string(),
            up: self.ip_v6_compatible,
        };

        vec![v4_status, v6_status]
    }

    fn into_gateway_status(self, pub_key: String, owner: String) -> Vec<GatewayStatus> {
        let v4_status = GatewayStatus {
            owner: owner.clone(),
            pub_key: pub_key.clone(),
            ip_version: "4".to_string(),
            up: self.ip_v4_compatible,
        };

        let v6_status = GatewayStatus {
            owner,
            pub_key,
            ip_version: "6".to_string(),
            up: self.ip_v6_compatible,
        };

        vec![v4_status, v6_status]
    }
}

#[derive(Default)]
pub(crate) struct TestReport {
    pub(crate) total_sent: usize,
    pub(crate) total_received: usize,
    pub(crate) malformed: Vec<InvalidNode>,

    // below are only populated if we're going to be printing the report
    pub(crate) only_ipv4_compatible_mixes: Vec<TestedNode>, // can't speak v6, but can speak v4
    pub(crate) only_ipv6_compatible_mixes: Vec<TestedNode>, // can't speak v4, but can speak v6
    pub(crate) completely_unroutable_mixes: Vec<TestedNode>, // can't speak either v4 or v6
    pub(crate) fully_working_mixes: Vec<TestedNode>,

    pub(crate) only_ipv4_compatible_gateways: Vec<TestedNode>, // can't speak v6, but can speak v4
    pub(crate) only_ipv6_compatible_gateways: Vec<TestedNode>, // can't speak v4, but can speak v6
    pub(crate) completely_unroutable_gateways: Vec<TestedNode>, // can't speak either v4 or v6
    pub(crate) fully_working_gateways: Vec<TestedNode>,
}

impl TestReport {
    fn print(&self, detailed: bool) {
        info!(target: "Test Report", "Sent total of {} packets", self.total_sent);
        info!(target: "Test Report", "Received total of {} packets", self.total_received);
        info!(target: "Test Report", "{} nodes are invalid", self.malformed.len());

        info!(target: "Test Report", "{} mixnodes speak ONLY IPv4 (NO IPv6 connectivity)", self.only_ipv4_compatible_mixes.len());
        info!(target: "Test Report", "{} mixnodes speak ONLY IPv6 (NO IPv4 connectivity)", self.only_ipv6_compatible_mixes.len());
        info!(target: "Test Report", "{} mixnodes are totally unroutable!", self.completely_unroutable_mixes.len());
        info!(target: "Test Report", "{} mixnodes work fine!", self.fully_working_mixes.len());

        info!(target: "Test Report", "{} gateways speak ONLY IPv4 (NO IPv6 connectivity)", self.only_ipv4_compatible_gateways.len());
        info!(target: "Test Report", "{} gateways speak ONLY IPv6 (NO IPv4 connectivity)", self.only_ipv6_compatible_gateways.len());
        info!(target: "Test Report", "{} gateways are totally unroutable!", self.completely_unroutable_gateways.len());
        info!(target: "Test Report", "{} gateways work fine!", self.fully_working_gateways.len());

        if detailed {
            info!(target: "Detailed report", "full summary:");
            for malformed in self.malformed.iter() {
                info!(target: "Invalid node", "{}", malformed)
            }

            for v4_node in self.only_ipv4_compatible_mixes.iter() {
                info!(target: "IPv4-only mixnode", "{}", v4_node)
            }

            for v6_node in self.only_ipv6_compatible_mixes.iter() {
                info!(target: "IPv6-only mixnode", "{}", v6_node)
            }

            for unroutable in self.completely_unroutable_mixes.iter() {
                info!(target: "Unroutable mixnode", "{}", unroutable)
            }

            for working in self.fully_working_mixes.iter() {
                info!(target: "Fully working mixnode", "{}", working)
            }

            for v4_node in self.only_ipv4_compatible_gateways.iter() {
                info!(target: "IPv4-only gateway", "{}", v4_node)
            }

            for v6_node in self.only_ipv6_compatible_gateways.iter() {
                info!(target: "IPv6-only gateway", "{}", v6_node)
            }

            for unroutable in self.completely_unroutable_gateways.iter() {
                info!(target: "Unroutable gateway", "{}", unroutable)
            }

            for working in self.fully_working_gateways.iter() {
                info!(target: "Fully working gateway", "{}", working)
            }
        }
    }

    fn parse_summary(&mut self, summary: &HashMap<TestedNode, NodeResult>) {
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
    pub(crate) batch_mix_status: BatchMixStatus,
    pub(crate) batch_gateway_status: BatchGatewayStatus,
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
        let mut summary: HashMap<TestedNode, NodeResult> = HashMap::new();

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

        let mix_statuses = mixes
            .into_iter()
            .flat_map(|(node, result)| {
                result
                    .into_mix_status(node.identity, node.owner)
                    .into_iter()
            })
            .collect();

        let gateway_statuses = gateways
            .into_iter()
            .flat_map(|(node, result)| {
                result
                    .into_gateway_status(node.identity, node.owner)
                    .into_iter()
            })
            .collect();

        TestSummary {
            batch_mix_status: BatchMixStatus {
                status: mix_statuses,
            },
            batch_gateway_status: BatchGatewayStatus {
                status: gateway_statuses,
            },
            test_report: report,
        }
    }
}
