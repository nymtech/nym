// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::UnixTimestamp;

// Internally used struct to catch results from the database to calculate uptimes for given mixnode/gateway
pub(crate) struct NodeStatus {
    pub(crate) timestamp: UnixTimestamp,
    pub(crate) up: bool,
}

// Internally used struct to catch results from the database to find active mixnodes/gateways
pub(crate) struct ActiveNode {
    pub(crate) id: i64,
    pub(crate) identity: String,
    pub(crate) owner: String,
}

pub(crate) struct RewardingReport {
    pub(crate) timestamp: UnixTimestamp,

    pub(crate) eligible_mixnodes: i64,
    pub(crate) eligible_gateways: i64,

    pub(crate) possibly_unrewarded_mixnodes: i64,
    pub(crate) possibly_unrewarded_gateways: i64,
}

pub(crate) struct FailedMixnodeRewardChunk {
    // references particular RewardingReport (there can be multiple chunks in a report)
    pub(crate) report_id: i64,
    pub(crate) error_message: String,
}

pub(crate) struct PossiblyUnrewardedMixnode {
    // references particular FailedMixnodeRewardChunk (there can be multiple nodes in a chunk)
    pub(crate) chunk_id: i64,
    pub(crate) identity: String,
    pub(crate) uptime: u8,
}

pub(crate) struct FailedGatewayRewardChunk {
    // references particular RewardingReport (there can be multiple chunks in a report)
    pub(crate) report_id: i64,
    pub(crate) error_message: String,
}

pub(crate) struct PossiblyUnrewardedGateway {
    // references particular FailedGatewayRewardChunk (there can be multiple nodes in a chunk)
    pub(crate) chunk_id: i64,
    pub(crate) identity: String,
    pub(crate) uptime: u8,
}
