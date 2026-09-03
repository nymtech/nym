// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::v3::{LivenessTestResult, StressTestResult};
use nym_api_requests::models::{LivenessScore, StressTestingScore, TestNode};
use nym_crypto::asymmetric::ed25519;
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

/// Row model for the `nym_node_stress_testing_result` table.
///
/// Produced from the wire-level [`StressTestResult`] via [`Self::from_submission`], which also
/// renames `test_performance` to `result` to match the on-disk column name and attaches the
/// submitting orchestrator's identity key so that `(node_id, test_timestamp, submitter_pubkey)`
/// dedupes retried at-least-once submissions. `testrun_id` is carried for traceability only and is
/// not part of any key.
#[derive(FromRow)]
pub struct NymNodeStressTestingResult {
    pub testrun_id: i64,
    pub submitter_pubkey: String,
    pub node_id: NodeId,
    pub result: f64,
    pub was_reachable: bool,
    pub test_timestamp: time::OffsetDateTime,
}

impl NymNodeStressTestingResult {
    pub fn from_submission(signer: &ed25519::PublicKey, value: StressTestResult) -> Self {
        NymNodeStressTestingResult {
            testrun_id: value.testrun_id,
            submitter_pubkey: signer.to_base58_string(),
            node_id: value.node_id,
            result: value.test_performance,
            was_reachable: value.was_reachable,
            test_timestamp: value.test_timestamp,
        }
    }
}

/// Row model for the `nym_node_liveness_result` table.
///
/// Same shape as [`NymNodeStressTestingResult`], and deliberately a distinct type rather than a
/// shared one with a kind column: the two kinds are separate streams with separate endpoints,
/// replay state, aggregation and weighting, so a shared row would need a discriminator that every
/// read must remember to filter on. The row carries no probed role - the submitted score is one
/// average whose shape is identical for a mixnode and a gateway. See the migration for why
/// `testrun_id` is not part of the key.
#[derive(FromRow)]
pub struct NymNodeLivenessResult {
    pub testrun_id: i64,
    pub submitter_pubkey: String,
    pub node_id: NodeId,
    pub result: f64,
    pub was_reachable: bool,
    pub test_timestamp: time::OffsetDateTime,
}

impl NymNodeLivenessResult {
    pub fn from_submission(signer: &ed25519::PublicKey, value: LivenessTestResult) -> Self {
        NymNodeLivenessResult {
            testrun_id: value.testrun_id,
            submitter_pubkey: signer.to_base58_string(),
            node_id: value.node_id,
            result: value.test_performance,
            was_reachable: value.was_reachable,
            test_timestamp: value.test_timestamp,
        }
    }
}

#[derive(FromRow)]
pub struct RetrievedAverageStressTestResult {
    pub node_id: NodeId,
    pub result: f64,
    pub was_reachable: bool,
}

impl From<RetrievedAverageStressTestResult> for StressTestingScore {
    fn from(value: RetrievedAverageStressTestResult) -> Self {
        StressTestingScore {
            score: value.result,
            was_reachable: value.was_reachable,
        }
    }
}

#[derive(FromRow)]
pub struct RetrievedAverageLivenessResult {
    pub node_id: NodeId,
    pub result: f64,
    pub was_reachable: bool,
}

impl From<RetrievedAverageLivenessResult> for LivenessScore {
    fn from(value: RetrievedAverageLivenessResult) -> Self {
        LivenessScore {
            score: value.result,
            was_reachable: value.was_reachable,
        }
    }
}
