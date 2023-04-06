// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_topology::NymTopology;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct NodeTesterRequest {
    pub(crate) id: u64,

    // specially constructed network topology that only contains the target
    // node on the tested layer
    pub(crate) testable_topology: NymTopology,
}

impl NodeTesterRequest {
    //
}
