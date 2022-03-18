// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Internally used struct to catch results from the database to calculate uptimes for given mixnode/gateway
pub(crate) struct NodeStatus {
    pub(crate) timestamp: i64,
    pub(crate) reliability: u8,
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

pub(crate) struct FailedMixnodeRewardChunk {
    // references particular interval_rewarding (there can be multiple chunks in a rewarding interval)
    pub(crate) interval_rewarding_id: i64,
    pub(crate) error_message: String,
}

pub(crate) struct PossiblyUnrewardedMixnode {
    // references particular FailedMixnodeRewardChunk (there can be multiple nodes in a chunk)
    pub(crate) chunk_id: i64,
    pub(crate) identity: String,
    pub(crate) uptime: u8,
}
