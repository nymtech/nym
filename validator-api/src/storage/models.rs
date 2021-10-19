// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::UnixTimestamp;

// Internally used struct to catch results from the database to calculate uptimes for given mixnode/gateway
pub(crate) struct NodeStatus {
    pub(crate) timestamp: UnixTimestamp,
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

pub(crate) struct EpochRewarding {
    #[allow(dead_code)]
    pub(crate) id: i64,
    #[allow(dead_code)]
    pub(crate) epoch_timestamp: i64,
    pub(crate) finished: bool,
}

pub(crate) struct RewardingReport {
    // references particular epoch_rewarding
    pub(crate) epoch_rewarding_id: i64,

    pub(crate) eligible_mixnodes: i64,
    pub(crate) eligible_gateways: i64,

    pub(crate) possibly_unrewarded_mixnodes: i64,
    pub(crate) possibly_unrewarded_gateways: i64,
}

pub(crate) struct FailedMixnodeRewardChunk {
    // references particular epoch_rewarding (there can be multiple chunks in a rewarding epoch)
    pub(crate) epoch_rewarding_id: i64,
    pub(crate) error_message: String,
}

pub(crate) struct PossiblyUnrewardedMixnode {
    // references particular FailedMixnodeRewardChunk (there can be multiple nodes in a chunk)
    pub(crate) chunk_id: i64,
    pub(crate) identity: String,
    pub(crate) uptime: u8,
}

pub(crate) struct FailedGatewayRewardChunk {
    // references particular epoch_rewarding (there can be multiple chunks in a rewarding epoch)
    pub(crate) epoch_rewarding_id: i64,
    pub(crate) error_message: String,
}

pub(crate) struct PossiblyUnrewardedGateway {
    // references particular FailedGatewayRewardChunk (there can be multiple nodes in a chunk)
    pub(crate) chunk_id: i64,
    pub(crate) identity: String,
    pub(crate) uptime: u8,
}
