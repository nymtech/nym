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
    pub(crate) monitor_run_id: i64,
    pub(crate) rounded_score: u8,
    pub(crate) nodes_count: u32,
}

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
}

#[derive(FromRow)]
pub(crate) struct ActiveGateway {
    pub(crate) id: i64,
    pub(crate) node_id: NodeId,
    pub(crate) identity: String,
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

// Simulated Rewarding System Models
// These models support comparison between old (24h cache-based) and new (1h route-based) rewarding

/// Represents a simulated reward epoch run
#[derive(FromRow, Debug, Clone)]
pub struct SimulatedRewardEpoch {
    pub id: i64,
    pub epoch_id: u32,
    pub calculation_method: String, // 'old', 'new', or 'comparison'
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub description: Option<String>,
    pub created_at: i64,
}

/// Node performance calculated using different methodologies
#[derive(FromRow, Debug, Clone)]
pub struct SimulatedNodePerformance {
    #[allow(dead_code)]
    pub id: i64,
    pub simulated_epoch_id: i64,
    pub node_id: NodeId,
    pub node_type: String, // 'mixnode' or 'gateway'
    pub identity_key: Option<String>,
    pub reliability_score: f64, // 0.0 to 100.0
    pub positive_samples: u32,
    pub negative_samples: u32,
    pub work_factor: Option<f64>,   // 0.0 to 1.0
    pub calculation_method: String, // 'old' or 'new'
    pub calculated_at: i64,
}

/// Performance comparison data for analyzing methodology differences
#[derive(FromRow, Debug, Clone)]
pub struct SimulatedPerformanceComparison {
    #[allow(dead_code)]
    pub id: i64,
    pub simulated_epoch_id: i64,
    pub node_id: NodeId,
    pub node_type: String,          // 'mixnode' or 'gateway'
    pub performance_score: f64,     // 0.0 to 100.0
    pub work_factor: f64,           // Work factor applied (e.g., 10.0 for active, 1.0 for standby)
    pub calculation_method: String, // 'old' or 'new'
    pub positive_samples: Option<i64>,
    pub negative_samples: Option<i64>,
    pub route_success_rate: Option<f64>, // 0.0 to 100.0, mainly for new method
    pub calculated_at: i64,
}

/// Performance ranking data for nodes
#[derive(FromRow, Debug, Clone)]
pub struct SimulatedPerformanceRanking {
    #[allow(dead_code)]
    pub id: i64,
    pub simulated_epoch_id: i64,
    pub node_id: NodeId,
    pub calculation_method: String,
    pub performance_rank: i64,
    pub performance_percentile: f64,
    #[allow(dead_code)]
    pub calculated_at: i64,
}

/// Route analysis metadata for simulation runs
#[derive(FromRow, Debug, Clone)]
pub struct SimulatedRouteAnalysis {
    #[allow(dead_code)]
    pub id: i64,
    pub simulated_epoch_id: i64,
    pub calculation_method: String, // 'old' or 'new'
    pub total_routes_analyzed: u32,
    pub successful_routes: u32,
    pub failed_routes: u32,
    pub average_route_reliability: Option<f64>, // 0.0 to 100.0
    pub time_window_hours: u32,                 // 1 for new method, 24 for old method
    pub analysis_parameters: Option<String>,    // JSON with analysis config
    pub calculated_at: i64,
}
