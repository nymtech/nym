// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::TestNode;
use nym_mixnet_contract_common::NodeId;
use sqlx::FromRow;
use time::Date;

#[derive(sqlx::FromRow, Debug, Clone, Copy)]
pub(crate) struct MonitorRunReport {
    #[allow(dead_code)]
    pub(crate) monitor_run_id: i64,
    pub(crate) network_reliability: f64,
    pub(crate) packets_sent: i64,
    pub(crate) packets_received: i64,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub(crate) struct MonitorRunScore {
    pub(crate) typ: String,
    #[allow(dead_code)]
    pub(crate) monitor_run_id: i64,
    pub(crate) rounded_score: u8,
    pub(crate) nodes_count: u32,
}

pub struct MixnodeDetails {
    pub id: i64,
    pub mix_id: i64,
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

#[derive(FromRow)]
pub struct GatewayDetailsBeforeMigration {
    pub id: i64,
    #[sqlx(default)]
    #[allow(dead_code)]
    pub node_id: Option<NodeId>,
    pub identity: String,
}

#[derive(FromRow)]
pub struct GatewayDetails {
    pub id: i64,
    pub node_id: NodeId,
    pub identity: String,
}

impl From<GatewayDetails> for TestNode {
    fn from(value: GatewayDetails) -> Self {
        TestNode {
            node_id: Some(value.node_id),
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

#[derive(FromRow)]
pub struct HistoricalUptime {
    #[allow(dead_code)]
    pub date: Date,
    pub uptime: i64,
}
