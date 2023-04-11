// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::topology::WasmNymTopology;
use node_tester_utils::TestMessage;
use nym_topology::NymTopology;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct TestMessageExt {
    pub test_id: u64,
}

impl TestMessageExt {
    pub fn new(test_id: u64) -> Self {
        TestMessageExt { test_id }
    }
}

#[wasm_bindgen]
pub struct NodeTesterRequest {
    pub(crate) test_msg: TestMessage<TestMessageExt>,

    // specially constructed network topology that only contains the target
    // node on the tested layer
    pub(crate) testable_topology: NymTopology,
}

#[wasm_bindgen]
impl NodeTesterRequest {
    pub fn injectable_topology(&self) -> WasmNymTopology {
        self.testable_topology.clone().into()
    }
}
