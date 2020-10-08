// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::test_packet::TestPacket;
use crate::test_run::RunInfo;
use directory_client::mixmining::{BatchMixStatus, MixStatus};
use log::*;
use std::collections::{HashMap, HashSet};
use std::mem;

pub(super) struct TestRun {
    print_report: bool,
    print_detailed_report: bool,
    test_report: TestReport,

    run_nonce: u64,
    expected_run_packets: HashSet<TestPacket>,
    received_packets: Vec<TestPacket>,
}

#[derive(Default)]
struct NodeResult {
    ip_v4_compatible: bool,
    ip_v6_compatible: bool,
}

#[derive(Default)]
struct TestReport {
    total_sent: usize,
    total_received: usize,
    malformed: Vec<String>,
    outdated: Vec<(String, String)>,

    // below are only populated if we're going to be printing the report
    only_ipv4_compatible: Vec<String>, // can't speak v6, but can speak v4
    only_ipv6_compatible: Vec<String>, // can't speak v4, but can speak v6
    completely_unroutable: Vec<String>, // can't speak either v4 or v6
    fully_working: Vec<String>,
}

impl TestReport {
    fn print(&self, detailed: bool) {
        info!(target: "Test Report", "Sent total of {} packets", self.total_sent);
        info!(target: "Test Report", "Received total of {} packets", self.total_received);
        info!(target: "Test Report", "{} nodes are malformed", self.malformed.len());
        info!(target: "Test Report", "{} nodes are outdated", self.outdated.len());
        info!(target: "Test Report", "{} nodes speak ONLY IPv4 (NO IPv6 connectivity)", self.only_ipv4_compatible.len());
        info!(target: "Test Report", "{} nodes speak ONLY IPv6 (NO IPv4 connectivity)", self.only_ipv6_compatible.len());
        info!(target: "Test Report", "{} nodes are totally unroutable!", self.completely_unroutable.len());
        info!(target: "Test Report", "{} nodes work fine!", self.fully_working.len());

        if detailed {
            info!(target: "Detailed report", "full summary:");
            for malformed in self.malformed.iter() {
                info!(target: "Malformed node", "{}", malformed)
            }
            for outdated in self.outdated.iter() {
                info!(target: "Outdated node", "{} (runs v{})", outdated.0, outdated.1)
            }
            for v4_node in self.only_ipv4_compatible.iter() {
                info!(target: "IPv4-only node", "{}", v4_node)
            }

            for v6_node in self.only_ipv6_compatible.iter() {
                info!(target: "IPv6-only node", "{}", v6_node)
            }

            for unroutable in self.completely_unroutable.iter() {
                info!(target: "Unroutable node", "{}", unroutable)
            }

            for working in self.fully_working.iter() {
                info!(target: "Fully working node", "{}", working)
            }
        }
    }
}

impl TestRun {
    pub(super) fn new(run_nonce: u64) -> Self {
        TestRun {
            print_report: false,
            print_detailed_report: false,
            test_report: Default::default(),
            run_nonce,
            expected_run_packets: Default::default(),
            received_packets: vec![],
        }
    }

    pub(super) fn with_report(mut self) -> Self {
        self.print_report = true;
        self
    }

    pub(super) fn with_detailed_report(mut self) -> Self {
        self.print_report = true;
        self.print_detailed_report = true;
        self
    }

    pub(super) fn refresh(&mut self, new_nonce: u64) {
        self.test_report = Default::default();
        self.run_nonce = new_nonce;
        self.expected_run_packets = Default::default();
        self.received_packets = Default::default();
    }

    fn down_status(&self, pub_key: String) -> Vec<MixStatus> {
        let v4_status = MixStatus {
            pub_key: pub_key.clone(),
            ip_version: "4".to_string(),
            up: false,
        };

        let v6_status = MixStatus {
            pub_key,
            ip_version: "6".to_string(),
            up: false,
        };

        let mut vec = Vec::with_capacity(2);
        vec.push(v4_status);
        vec.push(v6_status);
        vec
    }

    /// Update state of self based on the received `RunInfo`
    pub(super) fn start_run(&mut self, run_info: RunInfo) {
        if run_info.nonce != self.run_nonce {
            error!(
                "Received unexpected test run info! Got {}, expected: {}",
                self.run_nonce, run_info.nonce
            );
            return;
        }

        // notify about malformed nodes:
        for malformed_mix in run_info.malformed_mixes {
            debug!(
                target: "test-run",
                "{} is malformed", malformed_mix.clone()
            );
            self.test_report.malformed.push(malformed_mix);
        }

        for old_mix in run_info.incompatible_mixes {
            debug!(
                target: "test-run",
                "{} is outdated! It's on {} version",
                old_mix.0.clone(),
                old_mix.1
            );
            self.test_report.outdated.push(old_mix);
        }

        self.test_report.total_sent = run_info.test_packets.len();

        // store information about packets that are currently being sent
        self.expected_run_packets
            .reserve(run_info.test_packets.len());
        for test_packet in run_info.test_packets {
            self.expected_run_packets.insert(test_packet);
        }
    }

    pub(super) fn received_packet(&mut self, message: Vec<u8>) -> bool {
        let test_packet = match TestPacket::try_from_bytes(&message) {
            Ok(packet) => packet,
            Err(err) => {
                warn!("Invalid test packet received - {:?}", err);
                return false;
            }
        };

        if test_packet.nonce() == self.run_nonce {
            self.received_packets.push(test_packet);
        } else {
            warn!(
                "Received test packet for different test run! (Got {}, expected {})",
                test_packet.nonce(),
                self.run_nonce
            );
        }

        if self.received_packets.len() == self.expected_run_packets.len() {
            true
        } else {
            false
        }
    }

    fn produce_summary(&self) -> HashMap<String, NodeResult> {
        // contains map of all (seemingly valid) nodes and whether they speak ipv4/ipv6
        let mut summary: HashMap<String, NodeResult> = HashMap::new();

        // update based on data we actually get
        for received_status in self.received_packets.iter() {
            let entry = summary.entry(received_status.pub_key_string()).or_default();
            if received_status.ip_version().is_v4() {
                entry.ip_v4_compatible = true
            } else {
                entry.ip_v6_compatible = true
            }
        }

        // and then insert entries we didn't get but should have
        for expected in self.expected_run_packets.iter() {
            summary.entry(expected.pub_key_string()).or_default();
        }

        summary
    }

    fn finalize_report(&mut self) {
        let mut fully_working = Vec::new();
        let mut only_v4_compatible = Vec::new();
        let mut only_v6_compatible = Vec::new();
        let mut unroutable_nodes = Vec::new();

        let summary = self.produce_summary();
        for (node, result) in summary.into_iter() {
            if result.ip_v4_compatible && result.ip_v6_compatible {
                fully_working.push(node)
            } else if result.ip_v4_compatible {
                only_v4_compatible.push(node)
            } else if result.ip_v6_compatible {
                only_v6_compatible.push(node)
            } else {
                unroutable_nodes.push(node)
            }
        }

        self.test_report.fully_working = fully_working;
        self.test_report.only_ipv4_compatible = only_v4_compatible;
        self.test_report.only_ipv6_compatible = only_v6_compatible;
        self.test_report.completely_unroutable = unroutable_nodes;
    }

    pub(super) fn finish_run(&mut self) -> BatchMixStatus {
        self.test_report.total_received = self.received_packets.len();

        if self.print_report {
            self.finalize_report();
            self.test_report.print(self.print_detailed_report);
        }

        let mut mix_status = Vec::with_capacity(
            2 * (self.test_report.malformed.len() + self.test_report.outdated.len())
                + self.expected_run_packets.len(),
        );

        // firstly we know all malformed and outdated nodes are definitely down - we haven't sent
        // any test packets for those
        for malformed in self.test_report.malformed.iter() {
            let mut down_status = self.down_status(malformed.clone());
            mix_status.append(&mut down_status);
        }
        for outdated in self.test_report.outdated.iter() {
            let mut down_status = self.down_status(outdated.0.clone());
            mix_status.append(&mut down_status);
        }

        let mut undelivered = mem::replace(&mut self.expected_run_packets, HashSet::new());

        // then create status for packets we actually received
        for received in mem::replace(&mut self.received_packets, Vec::new()) {
            undelivered.remove(&received);
            mix_status.push(received.into_up_mixstatus())
        }

        // and finally create status for packets we sent but never received
        for undelivered_packet in undelivered.into_iter() {
            mix_status.push(undelivered_packet.into_down_mixstatus())
        }

        BatchMixStatus { status: mix_status }
    }
}
