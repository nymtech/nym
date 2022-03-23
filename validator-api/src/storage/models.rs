// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Internally used struct to catch results from the database to calculate uptimes for given mixnode/gateway
pub(crate) struct NodeStatus {
    pub timestamp: Option<i64>,
    pub reliability: Option<u8>,
}

impl NodeStatus {
    pub fn timestamp(&self) -> i64 {
        self.timestamp.unwrap_or_default()
    }

    pub fn reliability(&self) -> u8 {
        self.reliability.unwrap_or_default()
    }
}

// Internally used struct to catch results from the database to find active mixnodes/gateways
pub(crate) struct ActiveNode {
    pub(crate) id: i64,
    pub(crate) identity: String,
    pub(crate) owner: String,
}

pub(crate) struct TestingRoute {
    pub(crate) gateway_id: i64,
    pub(crate) layer1_mix_id: i64,
    pub(crate) layer2_mix_id: i64,
    pub(crate) layer3_mix_id: i64,
    pub(crate) monitor_run_id: i64,
}

pub(crate) struct RewardingReport {
    // references particular interval_rewarding
    pub(crate) interval_rewarding_id: i64,

    pub(crate) eligible_mixnodes: i64,

    pub(crate) possibly_unrewarded_mixnodes: i64,
}
