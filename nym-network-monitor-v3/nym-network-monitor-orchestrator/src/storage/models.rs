// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use nym_api_requests::models::network_monitor::StressTestResult;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::models::{
    self as api, LatencyDistribution, NymNodeData, TestRunData, TestRunInProgressData,
    TestRunResult,
};
use nym_node_requests::api::v1::node::models::NodeRoles;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NymNodeBond;
use std::time::Duration;
use time::OffsetDateTime;

/// Discriminator for the type of node targeted by a [`TestRun`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub(crate) enum TestType {
    #[default]
    Mixnode,
    Gateway,
}

/// Classification of a node based on the roles reported via its self-described endpoint.
/// [`NodeType::Unknown`] is used both as the initial value before the node is successfully
/// queried and when a queried node reports no roles at all.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub(crate) enum NodeType {
    #[default]
    Unknown,
    Mixnode,
    Gateway,
    MixnodeAndGateway,
}

impl NodeType {
    /// Classifies a node from the `NodeRoles` reported by its self-described endpoint.
    /// We key off `gateway_enabled` (entry-gateway capability) only — the `exit` property is
    /// not a useful distinction for test-target selection. A node reporting neither role maps
    /// to [`NodeType::Unknown`] and will be ignored by the mixnode testrun assignment query.
    pub(crate) fn from_roles(roles: &NodeRoles) -> Self {
        match (roles.mixnode_enabled, roles.gateway_enabled) {
            (true, true) => NodeType::MixnodeAndGateway,
            (true, false) => NodeType::Mixnode,
            (false, true) => NodeType::Gateway,
            (false, false) => NodeType::Unknown,
        }
    }
}

/// The data required to insert a new row into `testrun`. Does not carry an `id` since that
/// is assigned by the database on insertion.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NewTestRun {
    /// Contract-assigned node id of the node under test.
    pub(crate) node_id: i64,

    pub(crate) test_type: TestType,
    pub(crate) test_timestamp: OffsetDateTime,

    /// How long the test took, in microseconds.
    pub(crate) time_taken_us: i64,

    /// Noise handshake duration on the ingress (responder) side, in microseconds.
    pub(crate) ingress_noise_handshake_us: Option<i64>,

    /// Noise handshake duration on the egress (initiator) side, in microseconds.
    pub(crate) egress_noise_handshake_us: Option<i64>,

    /// Constant per-hop sphinx packet delay used during the test run, in microseconds.
    pub(crate) sphinx_packet_delay_us: i64,

    pub(crate) packets_sent: i64,
    pub(crate) packets_received: i64,

    /// RTT of the initial probe packet in microseconds. `None` if the probe did not complete.
    pub(crate) approximate_latency_us: Option<i64>,

    // RTT distribution over received packets (all NULL when no packets were received).
    pub(crate) packets_rtt_min_us: Option<i64>,
    pub(crate) packets_rtt_mean_us: Option<i64>,
    pub(crate) packets_rtt_median_us: Option<i64>,
    pub(crate) packets_rtt_max_us: Option<i64>,
    pub(crate) packets_rtt_std_dev_us: Option<i64>,

    // Batch send latency distribution (all NULL when no batches were sent).
    pub(crate) sending_latency_min_us: Option<i64>,
    pub(crate) sending_latency_mean_us: Option<i64>,
    pub(crate) sending_latency_median_us: Option<i64>,
    pub(crate) sending_latency_max_us: Option<i64>,
    pub(crate) sending_latency_std_dev_us: Option<i64>,

    pub(crate) received_duplicates: bool,

    /// First error that caused the test to abort. `None` if the run completed without error.
    pub(crate) error: Option<String>,
}

fn duration_to_us(d: std::time::Duration) -> i64 {
    d.as_micros() as i64
}

impl NewTestRun {
    /// Converts an API-level [`TestRunResult`] into a database-ready row,
    /// flattening [`LatencyDistribution`](nym_network_monitor_orchestrator_requests::models::LatencyDistribution)
    /// fields into individual microsecond columns and recording the current UTC time as the test timestamp.
    fn from_result(test_type: TestType, node_id: NodeId, result: TestRunResult) -> Self {
        NewTestRun {
            node_id: node_id as i64,
            test_type,
            test_timestamp: OffsetDateTime::now_utc(),
            time_taken_us: result.time_taken.as_micros() as i64,
            ingress_noise_handshake_us: result.ingress_noise_handshake.map(duration_to_us),
            egress_noise_handshake_us: result.egress_noise_handshake.map(duration_to_us),
            sphinx_packet_delay_us: duration_to_us(result.sphinx_packet_delay),
            packets_sent: result.packets_sent as i64,
            packets_received: result.packets_received as i64,
            approximate_latency_us: result.approximate_latency.map(duration_to_us),
            packets_rtt_min_us: result.packets_statistics.map(|s| duration_to_us(s.minimum)),
            packets_rtt_mean_us: result.packets_statistics.map(|s| duration_to_us(s.mean)),
            packets_rtt_median_us: result.packets_statistics.map(|s| duration_to_us(s.median)),
            packets_rtt_max_us: result.packets_statistics.map(|s| duration_to_us(s.maximum)),
            packets_rtt_std_dev_us: result
                .packets_statistics
                .map(|s| duration_to_us(s.standard_deviation)),
            sending_latency_min_us: result.sending_statistics.map(|s| duration_to_us(s.minimum)),
            sending_latency_mean_us: result.sending_statistics.map(|s| duration_to_us(s.mean)),
            sending_latency_median_us: result.sending_statistics.map(|s| duration_to_us(s.median)),
            sending_latency_max_us: result.sending_statistics.map(|s| duration_to_us(s.maximum)),
            sending_latency_std_dev_us: result
                .sending_statistics
                .map(|s| duration_to_us(s.standard_deviation)),
            received_duplicates: result.received_duplicates,
            error: result.error,
        }
    }

    /// Creates a new test run row for a mixnode stress test result.
    pub(crate) fn from_mixnode_result(node_id: NodeId, result: TestRunResult) -> Self {
        Self::from_result(TestType::Mixnode, node_id, result)
    }

    /// Creates a new test run row for a gateway stress test result.
    #[allow(dead_code)]
    pub(crate) fn from_gateway_result(node_id: NodeId, result: TestRunResult) -> Self {
        Self::from_result(TestType::Gateway, node_id, result)
    }
}

/// A row from the `testrun` table, as returned by a SELECT.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct TestRun {
    pub(crate) id: i64,

    #[sqlx(flatten)]
    pub(crate) inner: NewTestRun,
}

fn us_to_duration(us: i64) -> Duration {
    Duration::from_micros(us as u64)
}

/// Reassembles a [`LatencyDistribution`] from its four flattened microsecond columns.
/// Returns `None` if any column is `NULL`; the four columns are always all-set or all-NULL
/// together (see [`NewTestRun::from_result`]).
fn latency_distribution(
    min_us: Option<i64>,
    mean_us: Option<i64>,
    median_us: Option<i64>,
    max_us: Option<i64>,
    std_dev_us: Option<i64>,
) -> Option<LatencyDistribution> {
    match (min_us, mean_us, median_us, max_us, std_dev_us) {
        (Some(min), Some(mean), Some(median), Some(max), Some(std_dev)) => {
            Some(LatencyDistribution {
                minimum: us_to_duration(min),
                mean: us_to_duration(mean),
                median: us_to_duration(median),
                maximum: us_to_duration(max),
                standard_deviation: us_to_duration(std_dev),
            })
        }
        _ => None,
    }
}

/// Maps the internal enum onto its public API counterpart. Kept as a separate
/// type so `sqlx::Type` can be derived on the internal side without leaking
/// sqlx into the public request crate.
impl From<TestType> for api::TestType {
    fn from(t: TestType) -> Self {
        match t {
            TestType::Mixnode => api::TestType::Mixnode,
            TestType::Gateway => api::TestType::Gateway,
        }
    }
}

/// Lifts a `testrun` row into the public [`TestRunData`] shape: widens `i64`
/// ids/counters to the API's `u32`/`usize`, reconstitutes each
/// `LatencyDistribution` from its four microsecond columns, and converts
/// microsecond integers back into `std::time::Duration`.
impl From<TestRun> for TestRunData {
    fn from(run: TestRun) -> Self {
        let inner = run.inner;
        TestRunData {
            id: run.id,
            node_id: inner.node_id as u32,
            test_type: inner.test_type.into(),
            test_timestamp: inner.test_timestamp,
            result: TestRunResult {
                time_taken: Duration::from_micros(inner.time_taken_us as u64),
                ingress_noise_handshake: inner.ingress_noise_handshake_us.map(us_to_duration),
                egress_noise_handshake: inner.egress_noise_handshake_us.map(us_to_duration),
                sphinx_packet_delay: us_to_duration(inner.sphinx_packet_delay_us),
                packets_sent: inner.packets_sent as usize,
                packets_received: inner.packets_received as usize,
                approximate_latency: inner.approximate_latency_us.map(us_to_duration),
                packets_statistics: latency_distribution(
                    inner.packets_rtt_min_us,
                    inner.packets_rtt_mean_us,
                    inner.packets_rtt_median_us,
                    inner.packets_rtt_max_us,
                    inner.packets_rtt_std_dev_us,
                ),
                sending_statistics: latency_distribution(
                    inner.sending_latency_min_us,
                    inner.sending_latency_mean_us,
                    inner.sending_latency_median_us,
                    inner.sending_latency_max_us,
                    inner.sending_latency_std_dev_us,
                ),
                received_duplicates: inner.received_duplicates,
                error: inner.error,
            },
        }
    }
}

/// Projects a completed `testrun` row onto the nym-api's `StressTestResult` shape used by the
/// stress-test batch submission endpoint.
///
/// Two fields are synthesised here rather than stored directly:
///
/// - `test_performance` is `packets_received / packets_sent` clamped to `[0.0, 1.0]`. A run that
///   sent no packets collapses to `0.0`; `was_reachable` is the signal that lets the server tell
///   that case apart from a genuine zero score.
/// - `was_reachable` is `error.is_none()` — i.e. the test completed without an abort error. A run
///   that aborted before the node responded sets `error` to the first failure, so the inverse is
///   an accurate "did we reach the node at all" signal.
impl From<TestRun> for StressTestResult {
    fn from(run: TestRun) -> Self {
        let id = run.id;
        let inner = run.inner;
        let test_performance = if inner.packets_sent > 0 {
            (inner.packets_received as f64 / inner.packets_sent as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        StressTestResult {
            testrun_id: id,
            node_id: inner.node_id as u32,
            is_mixnode: matches!(inner.test_type, TestType::Mixnode),
            test_timestamp: inner.test_timestamp,
            test_performance,
            was_reachable: inner.error.is_none(),
        }
    }
}

/// The data required to insert or update a row in `nym_node`. Does not carry `last_testrun`
/// since that is managed separately via [`StorageManager::set_node_last_testrun`].
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NewNymNode {
    /// Node ID as assigned by the mixnet contract.
    pub(crate) node_id: i64,

    /// Ed25519 identity key, base58-encoded.
    /// A node_id always maps to exactly one identity_key and is never reassigned.
    pub(crate) identity_key: String,

    /// When this node was last observed as bonded in the contract.
    pub(crate) last_seen_bonded: OffsetDateTime,

    /// Mixnet socket address (host:port) at which the node accepts sphinx packets.
    /// Stored as a string; parse with `str::parse::<SocketAddr>()` when needed.
    pub(crate) mixnet_socket_address: Option<String>,

    /// X25519 public key used for Noise handshakes, base58-encoded.
    /// `None` if retrieval from the node failed.
    pub(crate) noise_key: Option<String>,

    /// Sphinx public key used for packet encryption, base58-encoded.
    /// `None` if retrieval from the node failed.
    /// Always `None`/`Some` together with `key_rotation_id`.
    pub(crate) sphinx_key: Option<String>,

    /// Key rotation epoch ID that `sphinx_key` belongs to.
    /// `None` if retrieval from the node failed.
    /// Always `None`/`Some` together with `sphinx_key`.
    pub(crate) key_rotation_id: Option<i64>,

    /// Classification of the node based on the roles reported via its self-described endpoint.
    /// [`NodeType::Unknown`] if the self-described retrieval failed.
    pub(crate) node_type: NodeType,
}

impl NewNymNode {
    pub(crate) fn from_bond(bond: &NymNodeBond) -> Self {
        NewNymNode {
            node_id: bond.node_id as i64,
            identity_key: bond.identity().to_string(),
            last_seen_bonded: OffsetDateTime::now_utc(),
            mixnet_socket_address: None,
            noise_key: None,
            sphinx_key: None,
            key_rotation_id: None,
            node_type: NodeType::Unknown,
        }
    }
}

/// A row from the `testrun_in_progress` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct TestRunInProgress {
    pub(crate) node_id: i64,
    pub(crate) started_at: OffsetDateTime,
}

/// Lifts a `testrun_in_progress` row into the public shape, narrowing `node_id`
/// from the sqlx-native `i64` to the API's `u32`.
impl From<TestRunInProgress> for TestRunInProgressData {
    fn from(row: TestRunInProgress) -> Self {
        TestRunInProgressData {
            node_id: row.node_id as u32,
            started_at: row.started_at,
        }
    }
}

/// A row from the `nym_node` table, as returned by a SELECT.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NymNode {
    #[sqlx(flatten)]
    pub(crate) inner: NewNymNode,

    /// ID of the most recent test run against this node. `None` if never tested.
    pub(crate) last_testrun: Option<i64>,
}

/// Decodes a node's stored base58 key strings and parses the socket address
/// into typed counterparts for the public API. Fails (with context) when any
/// stored value is malformed — this should not happen in practice because the
/// orchestrator writes these fields itself, so a failure here indicates
/// corruption or a schema regression and is surfaced as
/// [`crate::http::api::v1::error::ApiError::MalformedStoredData`] by callers.
impl TryFrom<NewNymNode> for NymNodeData {
    type Error = anyhow::Error;

    fn try_from(node: NewNymNode) -> anyhow::Result<Self> {
        let identity_key = ed25519::PublicKey::from_base58_string(&node.identity_key)
            .context("invalid identity_key")?;

        let mixnet_socket_address = node
            .mixnet_socket_address
            .map(|s| s.parse().context("invalid mixnet_socket_address"))
            .transpose()?;

        let noise_key = node
            .noise_key
            .map(|s| x25519::PublicKey::from_base58_string(&s).context("invalid noise_key"))
            .transpose()?;

        let sphinx_key = node
            .sphinx_key
            .map(|s| x25519::PublicKey::from_base58_string(&s).context("invalid sphinx_key"))
            .transpose()?;

        Ok(NymNodeData {
            node_id: node.node_id as u32,
            identity_key,
            last_seen_bonded: node.last_seen_bonded,
            mixnet_socket_address,
            noise_key,
            sphinx_key,
            key_rotation_id: node.key_rotation_id,
        })
    }
}

/// Convenience pass-through that drops `last_testrun` (callers that need the
/// latest run fetch it explicitly via [`TestRun`]) and delegates to the
/// [`NewNymNode`] conversion for the rest of the fields.
impl TryFrom<NymNode> for NymNodeData {
    type Error = anyhow::Error;

    fn try_from(node: NymNode) -> anyhow::Result<Self> {
        node.inner.try_into()
    }
}
