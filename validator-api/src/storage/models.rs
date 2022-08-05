// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use mixnet_contract_common::NodeId;

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

// Internally used structs to catch results from the database to find active mixnodes
pub(crate) struct ActiveMixnode {
    pub(crate) id: i64,
    pub(crate) mix_id: NodeId,
    pub(crate) identity_key: String,
    pub(crate) owner: String,
}

pub(crate) struct ActiveGateway {
    pub(crate) id: i64,
    pub(crate) identity: String,
    pub(crate) owner: String,
}

pub(crate) struct TestingRoute {
    pub(crate) gateway_db_id: i64,
    pub(crate) layer1_mix_db_id: i64,
    pub(crate) layer2_mix_db_id: i64,
    pub(crate) layer3_mix_db_id: i64,
    pub(crate) monitor_run_db_id: i64,
}

pub(crate) struct RewardingReport {
    // references particular interval_rewarding
    pub(crate) interval_rewarding_id: i64,

    pub(crate) eligible_mixnodes: i64,

    pub(crate) possibly_unrewarded_mixnodes: i64,
}
