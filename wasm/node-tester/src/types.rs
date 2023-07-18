// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_node_tester_utils::TestMessage;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_utils::console_log;

pub type NodeTestMessage = TestMessage<WasmTestMessageExt>;

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct WasmTestMessageExt {
    pub test_id: u32,
}

impl WasmTestMessageExt {
    pub fn new(test_id: u32) -> Self {
        WasmTestMessageExt { test_id }
    }
}

// TODO: maybe put it in the tester utils
#[wasm_bindgen]
pub struct NodeTestResult {
    pub sent_packets: u32,
    pub received_packets: u32,
    pub received_acks: u32,

    pub duplicate_packets: u32,
    pub duplicate_acks: u32,
}

impl Display for NodeTestResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test results: ")?;
        writeln!(f, "Total score: {:.2}%", self.score())?;
        writeln!(f, "Sent packets: {}", self.sent_packets)?;
        writeln!(f, "Received (valid) packets: {}", self.received_packets)?;
        writeln!(f, "Received (valid) acks: {}", self.received_acks)?;
        writeln!(f, "Received duplicate packets: {}", self.duplicate_packets)?;
        write!(f, "Received duplicate acks: {}", self.duplicate_acks)
    }
}

#[wasm_bindgen]
impl NodeTestResult {
    pub fn log_details(&self) {
        console_log!("{}", self)
    }

    pub fn score(&self) -> f32 {
        let expected = self.sent_packets * 2;
        let actual = (self.received_packets + self.received_acks)
            .saturating_sub(self.duplicate_packets + self.duplicate_acks);

        actual as f32 / expected as f32 * 100.
    }
}
