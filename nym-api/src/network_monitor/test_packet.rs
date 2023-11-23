// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_node_tester_utils::error::NetworkTestingError;
use nym_node_tester_utils::TestMessage;
use nym_topology::mix;
use serde::{Deserialize, Serialize};

pub(crate) type NodeTestMessage = TestMessage<NymApiTestMessageExt>;

#[derive(Serialize, Deserialize, Clone, Copy, Hash)]
pub(crate) struct NymApiTestMessageExt {
    pub(crate) route_id: u64,
    pub(crate) test_nonce: u64,
}

impl NymApiTestMessageExt {
    pub fn new(route_id: u64, test_nonce: u64) -> Self {
        NymApiTestMessageExt {
            route_id,
            test_nonce,
        }
    }

    pub fn mix_plaintexts(
        &self,
        node: &mix::Node,
        test_packets: u32,
    ) -> Result<Vec<Vec<u8>>, NetworkTestingError> {
        NodeTestMessage::mix_plaintexts(node, test_packets, *self)
    }
}
