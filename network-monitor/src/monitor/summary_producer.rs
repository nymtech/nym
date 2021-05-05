// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::monitor::preparer::InvalidNode;
use crate::node_status_api::{BatchMixStatus, MixStatus};
use crate::test_packet::TestPacket;
use crate::PENALISE_OUTDATED;
use crypto::asymmetric::identity;
use log::*;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct NodeResult {
    ip_v4_compatible: bool,
    ip_v6_compatible: bool,
}

impl NodeResult {
    fn into_mix_status(self, pub_key: String) -> Vec<MixStatus> {
        let v4_status = MixStatus {
            pub_key: pub_key.clone(),
            ip_version: "4".to_string(),
            up: self.ip_v4_compatible,
        };

        let v6_status = MixStatus {
            pub_key,
            ip_version: "6".to_string(),
            up: self.ip_v6_compatible,
        };

        vec![v4_status, v6_status]
    }
}

#[derive(Default)]
struct TestReport {
    total_sent: usize,
    total_received: usize,
    malformed: Vec<InvalidNode>,

    // below are only populated if we're going to be printing the report
    only_ipv4_compatible_mixes: Vec<String>, // can't speak v6, but can speak v4
    only_ipv6_compatible_mixes: Vec<String>, // can't speak v4, but can speak v6
    completely_unroutable_mixes: Vec<String>, // can't speak either v4 or v6
    fully_working_mixes: Vec<String>,

    only_ipv4_compatible_gateways: Vec<String>, // can't speak v6, but can speak v4
    only_ipv6_compatible_gateways: Vec<String>, // can't speak v4, but can speak v6
    completely_unroutable_gateways: Vec<String>, // can't speak either v4 or v6
    fully_working_gateways: Vec<String>,
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

    fn parse_summary(
        &mut self,
        summary: &HashMap<String, NodeResult>,
        all_gateways: HashSet<String>,
    ) {
        let is_gateway = |key: &str| all_gateways.contains(key);

        for (node, result) in summary.iter() {
            if is_gateway(&node) {
                if result.ip_v4_compatible && result.ip_v6_compatible {
                    self.fully_working_gateways.push(node.clone())
                } else if result.ip_v4_compatible {
                    self.only_ipv4_compatible_gateways.push(node.clone())
                } else if result.ip_v6_compatible {
                    self.only_ipv6_compatible_gateways.push(node.clone())
                } else {
                    self.completely_unroutable_gateways.push(node.clone())
                }
            } else if result.ip_v4_compatible && result.ip_v6_compatible {
                self.fully_working_mixes.push(node.clone())
            } else if result.ip_v4_compatible {
                self.only_ipv4_compatible_mixes.push(node.clone())
            } else if result.ip_v6_compatible {
                self.only_ipv6_compatible_mixes.push(node.clone())
            } else {
                self.completely_unroutable_mixes.push(node.clone())
            }
        }
    }
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
        expected_nodes: Vec<identity::PublicKey>,
        received_packets: Vec<TestPacket>,
        invalid_nodes: Vec<InvalidNode>,
        all_gateways: HashSet<String>,
    ) -> BatchMixStatus {
        let mut report = TestReport::default();

        // contains map of all (seemingly valid) nodes and whether they speak ipv4/ipv6
        let mut summary: HashMap<String, NodeResult> = HashMap::new();

        // update based on data we actually got
        for received_status in received_packets.iter() {
            let entry = summary.entry(received_status.pub_key_string()).or_default();
            if received_status.ip_version().is_v4() {
                entry.ip_v4_compatible = true
            } else {
                entry.ip_v6_compatible = true
            }
        }

        // insert entries we didn't get but were expecting
        for expected in expected_nodes.iter() {
            summary.entry(expected.to_base58_string()).or_default();
        }

        // finally insert malformed nodes
        for malformed in invalid_nodes.iter() {
            match malformed {
                InvalidNode::OutdatedMix(id, _) | InvalidNode::OutdatedGateway(id, _) => {
                    if PENALISE_OUTDATED {
                        summary.insert(id.to_string(), Default::default());
                    }
                }
                InvalidNode::MalformedMix(id) | InvalidNode::MalformedGateway(id) => {
                    summary.insert(id.to_string(), Default::default());
                }
            }
        }

        if self.print_report {
            report.total_sent = expected_nodes.len() * 2; // we sent two packets per node (one ipv4 and one ipv6)
            report.total_received = received_packets.len();
            report.malformed = invalid_nodes;
            report.parse_summary(&summary, all_gateways);
            report.print(self.print_detailed_report);
        }

        let status = summary
            .into_iter()
            .flat_map(|(key, result)| result.into_mix_status(key).into_iter())
            .collect();

        BatchMixStatus { status }
    }
}
