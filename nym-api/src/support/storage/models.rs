// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::TestNode;
use nym_mixnet_contract_common::MixId;

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
    pub(crate) mix_id: MixId,
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

// for now let's leave it here to have a data model to use with existing database tables
#[allow(unused)]
pub(crate) struct RewardingReport {
    // references particular interval_rewarding
    pub(crate) absolute_epoch_id: u32,

    pub(crate) eligible_mixnodes: u32,
}

pub struct MixnodeDetails {
    pub id: i64,
    pub mix_id: i64,
    pub owner: String,
    pub identity_key: String,
}

impl From<MixnodeDetails> for TestNode {
    fn from(value: MixnodeDetails) -> Self {
        TestNode {
            node_id: Some(value.mix_id.try_into().unwrap_or(u32::MAX)),
            identity_key: Some(value.identity_key),
        }
    }
}

pub struct GatewayDetails {
    pub id: i64,
    pub owner: String,
    pub identity: String,
}

impl From<GatewayDetails> for TestNode {
    fn from(value: GatewayDetails) -> Self {
        TestNode {
            node_id: None,
            identity_key: Some(value.identity),
        }
    }
}

pub struct TestedMixnodeStatus {
    pub db_id: i64,
    #[allow(dead_code)]
    pub mix_id: i64,
    #[allow(dead_code)]
    pub identity_key: String,
    pub reliability: Option<u8>,
    pub timestamp: i64,

    pub gateway_id: i64,
    pub layer1_mix_id: i64,
    pub layer2_mix_id: i64,
    pub layer3_mix_id: i64,
    pub monitor_run_id: i64,
}

pub struct TestedGatewayStatus {
    pub db_id: i64,
    #[allow(dead_code)]
    pub identity_key: String,
    pub reliability: Option<u8>,
    pub timestamp: i64,

    pub gateway_id: i64,
    pub layer1_mix_id: i64,
    pub layer2_mix_id: i64,
    pub layer3_mix_id: i64,
    pub monitor_run_id: i64,
}
