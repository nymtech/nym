// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{NodeTestResult, WasmTestMessageExt};
use futures::StreamExt;
use nym_node_tester_utils::processor::Received;
use nym_node_tester_utils::receiver::ReceivedReceiver;
use nym_node_tester_utils::FragmentIdentifier;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::MutexGuard as AsyncMutexGuard;
use wasm_utils::{console_error, console_log, console_warn};
use wasmtimer::tokio::sleep;

pub(crate) struct EphemeralTestReceiver<'a> {
    sent_packets: u32,
    expected_acks: HashSet<FragmentIdentifier>,

    received_valid_messages: HashSet<u32>,
    received_valid_acks: HashSet<FragmentIdentifier>,
    duplicate_packets: u32,
    duplicate_acks: u32,

    timeout_duration: Duration,
    receiver_permit: AsyncMutexGuard<'a, ReceivedReceiver<WasmTestMessageExt>>,
}

impl<'a> EphemeralTestReceiver<'a> {
    pub(crate) fn finish(self) -> NodeTestResult {
        NodeTestResult {
            sent_packets: self.sent_packets,
            received_packets: self.received_valid_messages.len() as u32,
            received_acks: self.received_valid_acks.len() as u32,
            duplicate_packets: self.duplicate_packets,
            duplicate_acks: self.duplicate_acks,
        }
    }

    pub(crate) fn new(
        sent_packets: u32,
        expected_acks: HashSet<FragmentIdentifier>,
        receiver_permit: AsyncMutexGuard<'a, ReceivedReceiver<WasmTestMessageExt>>,
        timeout: Duration,
    ) -> Self {
        EphemeralTestReceiver {
            sent_packets,
            expected_acks,
            received_valid_messages: Default::default(),
            received_valid_acks: Default::default(),
            duplicate_packets: 0,
            duplicate_acks: 0,
            timeout_duration: timeout,
            receiver_permit,
        }
    }

    fn on_next_received_packet(&mut self, packet: Option<Received<WasmTestMessageExt>>) -> bool {
        let Some(received_packet) = packet else {
            // can't do anything more...
            console_error!("packet receiver has stopped processing results!");
            return true;
        };
        match received_packet {
            Received::Message(msg) => {
                if !self.received_valid_messages.insert(msg.msg_id) {
                    self.duplicate_packets += 1;
                }
            }
            Received::Ack(frag_id) => {
                if self.expected_acks.contains(&frag_id) {
                    if !self.received_valid_acks.insert(frag_id) {
                        self.duplicate_acks += 1
                    }
                } else {
                    console_warn!("received an ack that was not part of the test! (id: {frag_id})")
                }
            }
        }

        if self.received_all() {
            console_log!("already received all the packets! finishing the test...");
            true
        } else {
            false
        }
    }

    fn received_all(&self) -> bool {
        self.received_valid_acks.len() == self.received_valid_messages.len()
            && self.received_valid_acks.len() == self.sent_packets as usize
    }

    pub(crate) async fn perform_test(mut self) -> NodeTestResult {
        let mut timeout_fut = sleep(self.timeout_duration);

        loop {
            tokio::select! {
                _ = &mut timeout_fut => {
                    console_warn!("reached test timeout before receiving all packets.");
                    break
                }
                received_packet = self.receiver_permit.next() => {
                    let is_done = self.on_next_received_packet(received_packet);
                    if is_done {
                        break
                    }
                }
            }
        }

        self.finish()
    }
}
