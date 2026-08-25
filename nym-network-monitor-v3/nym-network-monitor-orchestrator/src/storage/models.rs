// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::{Context, bail};
use nym_api_requests::models::v3 as nym_api_requests;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::models::{
    self as api, InterfaceMeasurement, LatencyDistribution, NymNodeData, TestRunData,
    TestRunInProgressData, TestRunResult,
};
use nym_node_requests::api::v1::node::models::NodeRoles;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NymNodeBond;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use strum::{Display, EnumCount, EnumIter};
use time::OffsetDateTime;

/// What a test run measures. Selects the run's cadence, eligibility rules and expected measurement
/// set, so - like its API counterpart - it deliberately has no `Default`: a silently defaulted kind
/// would measure the wrong thing rather than fail.
///
/// Every kind exists in order to be assigned, so the scheduler rotates over the variants themselves
/// rather than over a list kept in step with them by hand, and does so in DECLARATION ORDER. The
/// same holds for submission, where each kind is a stream of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, Display, EnumCount, EnumIter)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub(crate) enum TestKind {
    Stress,
    Liveness,
}

/// The role a node was probed in by a [`TestRun`]. Distinct from [`NodeType`], which is the
/// node's own capability classification and may be both. No `Default` for the same reason as
/// [`TestKind`]: a dual-role node is probed once per role, and defaulting would attribute one
/// role's measurement to the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub(crate) enum TestedRole {
    Mixnode,
    Gateway,
}

/// A (kind, role) combination the orchestrator can assign, and the key under which each one keeps
/// its own work state in `node_test_state`: its own staleness position and its own address rotation
/// cursor. That independence is the point of the type - a `mixnode_and_gateway` node is due
/// separately as a mixing hop and as a gateway, and neither pairing's run moves the other's clock.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TestPairing {
    pub(crate) test_kind: TestKind,
    pub(crate) tested_role: TestedRole,
}

impl TestPairing {
    pub(crate) const STRESS_MIXNODE: TestPairing = TestPairing {
        test_kind: TestKind::Stress,
        tested_role: TestedRole::Mixnode,
    };

    pub(crate) const LIVENESS_MIXNODE: TestPairing = TestPairing {
        test_kind: TestKind::Liveness,
        tested_role: TestedRole::Mixnode,
    };

    pub(crate) const LIVENESS_GATEWAY: TestPairing = TestPairing {
        test_kind: TestKind::Liveness,
        tested_role: TestedRole::Gateway,
    };
}

impl TestKind {
    /// The pairings this kind may assign, in the order that breaks a tie between two equally overdue
    /// ones. A kind's roles follow from what its probe measures: forwarding is performed only by a
    /// mixing hop, while the liveness probe has a shape for each role.
    pub(crate) fn pairings(&self) -> &'static [TestPairing] {
        match self {
            TestKind::Stress => &[TestPairing::STRESS_MIXNODE],
            TestKind::Liveness => &[TestPairing::LIVENESS_MIXNODE, TestPairing::LIVENESS_GATEWAY],
        }
    }
}

/// Which of the node's packet-handling interfaces a [`TestRunMeasurement`] describes. Names the
/// node function exercised rather than a route; the test kind never appears here, since it is a
/// property of the run and lives on the parent row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub(crate) enum ExercisedInterface {
    MixForwarding,
    ClientIngest,
    ClientDelivery,
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
    /// to [`NodeType::Unknown`] and will be ignored by every kind's assignment query.
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
/// is assigned by the database on insertion, nor the run's measurements, which are separate rows
/// (see [`TestRunMeasurement`]).
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NewTestRun {
    /// Contract-assigned node id of the node under test.
    pub(crate) node_id: i64,

    /// What this run measured.
    pub(crate) test_kind: TestKind,

    /// Which role of the node this run probed.
    pub(crate) tested_role: TestedRole,

    /// The address of that node that was tested, as reported by the agent that performed the run.
    pub(crate) tested_address: String,

    pub(crate) test_timestamp: OffsetDateTime,

    /// How long the test took, in microseconds. Run-level rather than per-measurement: a gateway
    /// run holds one session open across both of its phases, so the two cannot be timed apart.
    pub(crate) time_taken_us: i64,

    /// First error that caused the test to abort. `None` if the run completed without error.
    pub(crate) error: Option<String>,
}

fn duration_to_us(d: Duration) -> i64 {
    d.as_micros() as i64
}

fn us_to_duration(us: i64) -> Duration {
    Duration::from_micros(us as u64)
}

impl NewTestRun {
    /// Converts an API-level [`TestRunResult`] into the run-level database row, recording the
    /// current UTC time as the test timestamp. The result's measurements are converted separately
    /// via [`TestRunMeasurement::from`].
    ///
    /// `test_kind` and `tested_role` are taken as arguments rather than read from the result: both
    /// come from the `testrun_in_progress` row the orchestrator stamped when it dispatched the run,
    /// which is authoritative precisely because it is the value the orchestrator chose. The result
    /// carries a kind of its own, but it is the agent's echo of that same value, so it is not read
    /// here.
    pub(crate) fn from_result(
        node_id: NodeId,
        tested_address: SocketAddr,
        test_kind: TestKind,
        tested_role: TestedRole,
        result: &TestRunResult,
    ) -> Self {
        NewTestRun {
            node_id: node_id as i64,
            test_kind,
            tested_role,
            tested_address: tested_address.to_string(),
            test_timestamp: OffsetDateTime::now_utc(),
            time_taken_us: duration_to_us(result.time_taken),
            error: result.error.clone(),
        }
    }
}

/// A row from the `testrun` table, as returned by a SELECT.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct TestRun {
    pub(crate) id: i64,

    #[sqlx(flatten)]
    pub(crate) inner: NewTestRun,
}

/// The counts and timings gathered against ONE of a node's interfaces: a row of
/// `testrun_measurement` minus its `testrun_id`, which the parent run supplies.
///
/// A mixnode probe of either kind produces exactly one of these; a gateway liveness run produces
/// one per phase, kept apart so a healthy ingest with a dead delivery stays distinguishable from a
/// uniformly half-lossy node.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct TestRunMeasurement {
    /// Which interface these counts describe.
    pub(crate) interface: ExercisedInterface,

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
}

/// A `testrun_measurement` row carrying the run it belongs to, for the batched read that fetches
/// the measurements of a whole page of runs at once and groups them by parent.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct KeyedTestRunMeasurement {
    pub(crate) testrun_id: i64,

    #[sqlx(flatten)]
    pub(crate) inner: TestRunMeasurement,
}

/// Flattens an API-level measurement into its microsecond columns.
impl From<&InterfaceMeasurement> for TestRunMeasurement {
    fn from(measurement: &InterfaceMeasurement) -> Self {
        TestRunMeasurement {
            interface: measurement.interface.into(),
            ingress_noise_handshake_us: measurement.ingress_noise_handshake.map(duration_to_us),
            egress_noise_handshake_us: measurement.egress_noise_handshake.map(duration_to_us),
            sphinx_packet_delay_us: duration_to_us(measurement.sphinx_packet_delay),
            packets_sent: measurement.packets_sent as i64,
            packets_received: measurement.packets_received as i64,
            approximate_latency_us: measurement.approximate_latency.map(duration_to_us),
            packets_rtt_min_us: measurement
                .packets_statistics
                .map(|s| duration_to_us(s.minimum)),
            packets_rtt_mean_us: measurement
                .packets_statistics
                .map(|s| duration_to_us(s.mean)),
            packets_rtt_median_us: measurement
                .packets_statistics
                .map(|s| duration_to_us(s.median)),
            packets_rtt_max_us: measurement
                .packets_statistics
                .map(|s| duration_to_us(s.maximum)),
            packets_rtt_std_dev_us: measurement
                .packets_statistics
                .map(|s| duration_to_us(s.standard_deviation)),
            sending_latency_min_us: measurement
                .sending_statistics
                .map(|s| duration_to_us(s.minimum)),
            sending_latency_mean_us: measurement
                .sending_statistics
                .map(|s| duration_to_us(s.mean)),
            sending_latency_median_us: measurement
                .sending_statistics
                .map(|s| duration_to_us(s.median)),
            sending_latency_max_us: measurement
                .sending_statistics
                .map(|s| duration_to_us(s.maximum)),
            sending_latency_std_dev_us: measurement
                .sending_statistics
                .map(|s| duration_to_us(s.standard_deviation)),
            received_duplicates: measurement.received_duplicates,
        }
    }
}

/// Reassembles a [`LatencyDistribution`] from its five flattened microsecond columns.
/// Returns `None` if any column is `NULL`; the five columns are always all-set or all-NULL
/// together (see [`TestRunMeasurement::from`]).
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

/// Lifts a stored measurement back into its API shape: widens the counters and turns each
/// microsecond integer group back into a [`Duration`] or a [`LatencyDistribution`].
impl From<&TestRunMeasurement> for InterfaceMeasurement {
    fn from(measurement: &TestRunMeasurement) -> Self {
        InterfaceMeasurement {
            interface: measurement.interface.into(),
            ingress_noise_handshake: measurement.ingress_noise_handshake_us.map(us_to_duration),
            egress_noise_handshake: measurement.egress_noise_handshake_us.map(us_to_duration),
            sphinx_packet_delay: us_to_duration(measurement.sphinx_packet_delay_us),
            packets_sent: measurement.packets_sent as usize,
            packets_received: measurement.packets_received as usize,
            approximate_latency: measurement.approximate_latency_us.map(us_to_duration),
            packets_statistics: latency_distribution(
                measurement.packets_rtt_min_us,
                measurement.packets_rtt_mean_us,
                measurement.packets_rtt_median_us,
                measurement.packets_rtt_max_us,
                measurement.packets_rtt_std_dev_us,
            ),
            sending_statistics: latency_distribution(
                measurement.sending_latency_min_us,
                measurement.sending_latency_mean_us,
                measurement.sending_latency_median_us,
                measurement.sending_latency_max_us,
                measurement.sending_latency_std_dev_us,
            ),
            received_duplicates: measurement.received_duplicates,
        }
    }
}

// The internal enums exist as separate types so `sqlx::Type` can be derived without leaking sqlx
// into the public request crate; these conversions are the only bridge between the two.

impl From<TestKind> for api::TestKind {
    fn from(kind: TestKind) -> Self {
        match kind {
            TestKind::Stress => api::TestKind::Stress,
            TestKind::Liveness => api::TestKind::Liveness,
        }
    }
}

impl From<TestedRole> for api::TestedRole {
    fn from(role: TestedRole) -> Self {
        match role {
            TestedRole::Mixnode => api::TestedRole::Mixnode,
            TestedRole::Gateway => api::TestedRole::Gateway,
        }
    }
}

impl From<ExercisedInterface> for api::ExercisedInterface {
    fn from(interface: ExercisedInterface) -> Self {
        match interface {
            ExercisedInterface::MixForwarding => api::ExercisedInterface::MixForwarding,
            ExercisedInterface::ClientIngest => api::ExercisedInterface::ClientIngest,
            ExercisedInterface::ClientDelivery => api::ExercisedInterface::ClientDelivery,
        }
    }
}

impl From<api::ExercisedInterface> for ExercisedInterface {
    fn from(interface: api::ExercisedInterface) -> Self {
        match interface {
            api::ExercisedInterface::MixForwarding => ExercisedInterface::MixForwarding,
            api::ExercisedInterface::ClientIngest => ExercisedInterface::ClientIngest,
            api::ExercisedInterface::ClientDelivery => ExercisedInterface::ClientDelivery,
        }
    }
}

/// The nym-api carries its own copy of the same enum - the dependency runs orchestrator ->
/// nym-api-requests, so the two cannot share one type - which makes this the bridge the liveness
/// submission crosses.
impl From<ExercisedInterface> for nym_api_requests::ExercisedInterface {
    fn from(interface: ExercisedInterface) -> Self {
        match interface {
            ExercisedInterface::MixForwarding => {
                nym_api_requests::ExercisedInterface::MixForwarding
            }
            ExercisedInterface::ClientIngest => nym_api_requests::ExercisedInterface::ClientIngest,
            ExercisedInterface::ClientDelivery => {
                nym_api_requests::ExercisedInterface::ClientDelivery
            }
        }
    }
}

/// A completed run together with every measurement it produced, i.e. the parent row plus its
/// children reassembled. This is the unit both the operator read surface and the nym-api
/// submission path consume, since a run's score is defined over its whole measurement set.
#[derive(Debug, Clone)]
pub(crate) struct CompletedTestRun {
    pub(crate) run: TestRun,
    pub(crate) measurements: Vec<TestRunMeasurement>,
}

impl CompletedTestRun {
    /// The measurement for a given interface, if this run exercised it.
    pub(crate) fn measurement(&self, interface: ExercisedInterface) -> Option<&TestRunMeasurement> {
        self.measurements
            .iter()
            .find(|measurement| measurement.interface == interface)
    }

    /// Delivery ratio against one interface, zero if the run produced no measurement for it.
    fn performance(&self, interface: ExercisedInterface) -> f64 {
        self.measurement(interface)
            // the ratio (and its clamp) is defined once, on the API-level measurement
            .map(|measurement| InterfaceMeasurement::from(measurement).received_ratio())
            .unwrap_or_default()
    }
}

/// Lifts a completed run into the public [`TestRunData`] shape: widens `i64` ids to the API's
/// `u32`, converts microsecond integers back into `std::time::Duration`, and reattaches the
/// measurements.
impl From<CompletedTestRun> for TestRunData {
    fn from(completed: CompletedTestRun) -> Self {
        let measurements = completed.measurements.iter().map(Into::into).collect();
        let run = completed.run;
        let inner = run.inner;

        TestRunData {
            id: run.id,
            node_id: inner.node_id as u32,
            // a malformed stored address is not worth failing the whole result over,
            // it's informational rather than something we act on
            tested_address: inner.tested_address.parse().ok(),
            tested_role: inner.tested_role.into(),
            test_timestamp: inner.test_timestamp,
            result: TestRunResult {
                kind: inner.test_kind.into(),
                time_taken: us_to_duration(inner.time_taken_us),
                error: inner.error,
                measurements,
            },
        }
    }
}

/// Projects a completed stress run onto the nym-api's `StressTestResult` shape used by the
/// stress-test batch submission endpoint.
///
/// Two fields are synthesised here rather than stored directly:
///
/// - `test_performance` is the delivery ratio of the run's `mix_forwarding` measurement, which is
///   the only interface a stress run exercises. A run that saw duplicate packets scores `0.0`
///   outright: an honest node never replays a packet, so the whole measurement is discarded. A run
///   that sent no packets also collapses to `0.0`; `was_reachable` is what lets the server tell
///   that case apart from a genuine zero score.
/// - `was_reachable` is `error.is_none()` — i.e. the test completed without an abort error. A run
///   that aborted before the node responded sets `error` to the first failure, so the inverse is
///   an accurate "did we reach the node at all" signal.
impl From<&CompletedTestRun> for nym_api_requests::StressTestResult {
    fn from(completed: &CompletedTestRun) -> Self {
        let inner = &completed.run.inner;

        let test_performance = match completed.measurement(ExercisedInterface::MixForwarding) {
            Some(measurement) if !measurement.received_duplicates => {
                // the ratio (and its clamp) is defined once, on the API-level measurement
                InterfaceMeasurement::from(measurement).received_ratio()
            }
            _ => 0.0,
        };

        nym_api_requests::StressTestResult {
            testrun_id: completed.run.id,
            node_id: inner.node_id as u32,
            is_mixnode: matches!(inner.tested_role, TestedRole::Mixnode),
            test_timestamp: inner.test_timestamp,
            test_performance,
            was_reachable: inner.error.is_none(),
        }
    }
}

/// Projects a completed liveness run onto the nym-api's `LivenessTestResult` shape.
///
/// The score averages over the interfaces the probe is EXPECTED to produce rather than over the
/// ones that came back, so a phase that produced nothing scores zero instead of shrinking the
/// denominator - a gateway whose delivery never ran must not tie with one that passed both. The
/// breakdown is built from the same set, so it always accounts for the score above it, and the role
/// that fixes that set never leaves the orchestrator: the ratio is already normalised into
/// `[0.0, 1.0]` and comparable across roles without it. `was_reachable` is `error.is_none()`, as on
/// the stress path.
impl From<&CompletedTestRun> for nym_api_requests::LivenessTestResult {
    fn from(completed: &CompletedTestRun) -> Self {
        let inner = &completed.run.inner;

        let expected: &[ExercisedInterface] = match inner.tested_role {
            TestedRole::Mixnode => &[ExercisedInterface::MixForwarding],
            TestedRole::Gateway => &[
                ExercisedInterface::ClientIngest,
                ExercisedInterface::ClientDelivery,
            ],
        };

        let interfaces: Vec<_> = expected
            .iter()
            .map(|&interface| nym_api_requests::InterfacePerformance {
                interface: interface.into(),
                performance: completed.performance(interface),
            })
            .collect();

        nym_api_requests::LivenessTestResult {
            testrun_id: completed.run.id,
            node_id: inner.node_id as u32,
            test_timestamp: inner.test_timestamp,
            test_performance: interfaces.iter().map(|i| i.performance).sum::<f64>()
                / expected.len() as f64,
            was_reachable: inner.error.is_none(),
            interfaces,
        }
    }
}

/// The data required to insert or update a row in `nym_node`. Carries no test state: staleness,
/// the rotation pointer and the last run all live in [`NodeTestState`], keyed per (kind, role).
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

    /// Every ip address announced by the node, comma-separated.
    /// `None` if retrieval from the node failed.
    pub(crate) announced_ips: Option<String>,

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

    /// Port of the node's PLAIN client websocket listener, which a gateway liveness probe opens
    /// its client session against. `None` for a node announcing no entry-gateway interface, and
    /// for one that has never been successfully queried.
    pub(crate) clients_ws_port: Option<i64>,
}

/// What is known about a node from its on-chain bond alone, i.e. without its own endpoint having
/// answered. Written on its own when a refresh could not describe the node, so that the bond is
/// still recorded without disturbing anything learned in an earlier cycle.
pub(crate) struct BondedNymNode {
    pub(crate) node_id: i64,
    pub(crate) identity_key: String,
    pub(crate) last_seen_bonded: OffsetDateTime,
}

impl BondedNymNode {
    pub(crate) fn from_bond(bond: &NymNodeBond) -> Self {
        BondedNymNode {
            node_id: bond.node_id as i64,
            identity_key: bond.identity().to_string(),
            last_seen_bonded: OffsetDateTime::now_utc(),
        }
    }
}

/// A row from the `nym_node` table, as returned by a SELECT.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NymNode {
    #[sqlx(flatten)]
    pub(crate) inner: NewNymNode,
}

impl NymNode {
    /// Every ip address the node announced, falling back to the one in `mixnet_socket_address` for
    /// nodes that haven't been refreshed since `announced_ips` was introduced. Unparseable entries
    /// are skipped rather than failing the whole assignment.
    ///
    /// The stored set is canonicalised, deduplicated and sorted on write (see
    /// [`NodeRefresher`](crate::orchestrator::node_refresher::NodeRefresher)), which is what makes
    /// the [`next_ip_to_test`] rotation stable across refreshes.
    pub(crate) fn announced_ips(&self) -> Vec<IpAddr> {
        let announced: Vec<_> = self
            .inner
            .announced_ips
            .iter()
            .flat_map(|ips| ips.split(','))
            .filter_map(|ip| ip.trim().parse().ok())
            .collect();

        if !announced.is_empty() {
            return announced;
        }

        self.inner
            .mixnet_socket_address
            .as_ref()
            .and_then(|addr| addr.parse::<SocketAddr>().ok())
            .map(|addr| vec![addr.ip()])
            .unwrap_or_default()
    }
}

/// The ip a given (kind, role) pairing should test next: the one following `previously_tested_ip`
/// in `announced`, so consecutive runs of that pairing rotate through every address the node has.
/// Falls back to the first announced address when the pointer is unset (the pairing has never
/// assigned this node) or no longer announced.
///
/// The announced set belongs to the node while the pointer belongs to the pairing, which is what
/// lets two kinds - or the two roles of one dual-role node - advance over the same set
/// independently instead of skipping addresses because of each other.
pub(crate) fn next_ip_to_test(
    announced: &[IpAddr],
    previously_tested_ip: Option<&str>,
) -> Option<IpAddr> {
    let previous = previously_tested_ip.and_then(|ip| ip.parse::<IpAddr>().ok());
    let previous_index = previous.and_then(|ip| announced.iter().position(|a| *a == ip));

    match previous_index {
        Some(index) => announced.get((index + 1) % announced.len()).copied(),
        None => announced.first().copied(),
    }
}

/// A row from the `node_test_state` table: what one (node, kind, role) pairing has done so far.
///
/// Every column beyond the key is nullable because a row is created by whichever path touches the
/// pairing first — the assignment writes only [`Self::last_tested_ip`], the result submission only
/// [`Self::last_tested_at`] and [`Self::last_testrun_id`].
// written by the insert and assignment paths as individual columns; read back as a whole row by
// the per-pairing rotation and staleness tests
#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NodeTestState {
    pub(crate) node_id: i64,
    pub(crate) test_kind: TestKind,
    pub(crate) tested_role: TestedRole,

    /// When this pairing last completed a run against the node, which is what the staleness gate
    /// reads. `None` while the node has only ever been assigned, never measured. Stored directly
    /// rather than joined through [`Self::last_testrun_id`] so that evicting an old result does not
    /// make the node read as never-tested and jump the assignment queue.
    pub(crate) last_tested_at: Option<OffsetDateTime>,

    /// The most recent completed run of this pairing, or `None` once that run has been evicted.
    pub(crate) last_testrun_id: Option<i64>,

    /// The address handed out for this pairing's most recent assignment, i.e. its rotation pointer
    /// into the node's announced set. Advances when the assignment is handed out rather than when a
    /// result arrives, so an abandoned run still moves the node onto its next address.
    pub(crate) last_tested_ip: Option<String>,
}

/// A row from the `testrun_in_progress` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct TestRunInProgress {
    pub(crate) node_id: i64,
    pub(crate) started_at: OffsetDateTime,

    /// When the lease expires and the row becomes reapable, materialised at dispatch as
    /// `started_at` plus the kind's lease budget so the eviction sweep never has to learn about
    /// kinds.
    // compared in SQL by the eviction sweep rather than in Rust; surfaced on the operator read
    // surface alongside the kind and role
    #[allow(dead_code)]
    pub(crate) expires_at: OffsetDateTime,

    /// What the run was dispatched to measure, and against which role of the node. This is the
    /// authoritative source of both when the result comes back: the submission reports only the
    /// node and the address, so reading them from here is what keeps the orchestrator from
    /// trusting an agent's echo of values the orchestrator itself chose.
    pub(crate) test_kind: TestKind,
    pub(crate) tested_role: TestedRole,
}

/// Lifts a `testrun_in_progress` row into the public shape, narrowing `node_id`
/// from the sqlx-native `i64` to the API's `u32`. The lease, kind and role are deliberately not
/// surfaced yet; exposing them on the operator read surface belongs with the rest of that work.
impl From<TestRunInProgress> for TestRunInProgressData {
    fn from(row: TestRunInProgress) -> Self {
        TestRunInProgressData {
            node_id: row.node_id as u32,
            started_at: row.started_at,
        }
    }
}

/// A candidate row from the assignment query: the node, joined onto the rotation pointer of the
/// (kind, role) pairing being assigned. Only the pointer is taken from the state side, which is
/// what keeps each pairing's rotation independent of every other.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct AssignmentCandidate {
    #[sqlx(flatten)]
    pub(crate) node: NymNode,

    pub(crate) last_tested_ip: Option<String>,
}

/// How overdue the node a pairing would assign next is, i.e. the key the role selection within one
/// kind compares.
///
/// `Ord` comes from the declaration order and then from the timestamp, so `NeverTested` outranks
/// every measured node and an older measurement outranks a newer one - the same ordering the
/// assignment query applies through `NULLS FIRST`, which is what makes the most overdue head the
/// minimum.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PairingHead {
    NeverTested,
    LastTestedAt(OffsetDateTime),
}

/// What the scheduler settled on for one pairing, expressed in the durations its configuration
/// carries. Resolved into an [`AssignmentRequest`] against a single `now`.
#[derive(Debug, Copy, Clone)]
pub(crate) struct PairingSchedule {
    pub(crate) pairing: TestPairing,

    /// Minimum time since this pairing's last run against a node before it is due again.
    pub(crate) staleness_age: Duration,

    /// How long a dispatched run holds its node before the lease expires.
    pub(crate) lease_budget: Duration,

    /// Upper bound on the targets one assignment may carry: one for a stress run, the role's wave
    /// size for a liveness run.
    pub(crate) wave_size: usize,
}

impl PairingSchedule {
    /// The stress pairing. Its wave is always ONE target, since a stress assignment carries a single
    /// probe target by construction.
    pub(crate) fn stress(staleness_age: Duration, lease_budget: Duration) -> Self {
        PairingSchedule {
            pairing: TestPairing::STRESS_MIXNODE,
            staleness_age,
            lease_budget,
            wave_size: 1,
        }
    }
}

/// One pairing's dispatch parameters with every duration already resolved against one `now`, which
/// is what the assignment query binds. Absolute rather than relative so a caller can hold a single
/// timestamp across the gates it applies and the rows it stamps.
#[derive(Debug, Copy, Clone)]
pub(crate) struct AssignmentRequest {
    pub(crate) pairing: TestPairing,

    /// Stamped as `started_at` on every in-progress row this assignment writes.
    pub(crate) now: OffsetDateTime,

    /// Staleness gate: a node this pairing has tested before is eligible only if that run predates
    /// this. Never-tested nodes bypass it.
    pub(crate) last_tested_before: OffsetDateTime,

    /// Lease deadline stamped on every in-progress row, so the eviction sweep needs no knowledge of
    /// which kind produced the row.
    pub(crate) expires_at: OffsetDateTime,

    /// Maximum number of targets to select and lock.
    pub(crate) wave_size: usize,
}

/// A node selected for a test run, along with the address that this particular run should target.
pub(crate) struct AssignedTestrun {
    pub(crate) node: NymNode,

    /// The announced ip picked for this run by [`next_ip_to_test`].
    pub(crate) tested_ip: IpAddr,
}

impl AssignedTestrun {
    /// The target an agent probes over the node's mixnet listener: the stored keys decoded, and the
    /// address this run rotated onto carrying the node's announced mix port.
    ///
    /// Every field it needs is one the assignment query filters on, so a missing one means
    /// corruption or a schema regression rather than an untestable node - the same relationship
    /// [`NymNodeData`]'s conversion has to its stored row, and reported the same way.
    pub(crate) fn mixnet_probe_target(&self) -> anyhow::Result<api::MixnetProbeTarget> {
        let node = &self.node.inner;

        let identity_key = ed25519::PublicKey::from_base58_string(&node.identity_key)
            .context("invalid identity_key")?;

        let (Some(address), Some(noise_key), Some(sphinx_key), Some(key_rotation_id)) = (
            node.mixnet_socket_address.as_deref(),
            node.noise_key.as_deref(),
            node.sphinx_key.as_deref(),
            node.key_rotation_id,
        ) else {
            bail!(
                "node {} was assigned for testing without its complete data",
                node.node_id
            )
        };

        // the stored socket address only contributes the mix port - the address under test comes
        // from the rotation over everything the node announced
        let mix_port = address
            .parse::<SocketAddr>()
            .context("invalid mixnet_socket_address")?
            .port();

        Ok(api::MixnetProbeTarget {
            node_id: node.node_id as u32,
            identity_key,
            node_address: SocketAddr::new(self.tested_ip, mix_port),
            node_ips: self.node.announced_ips(),
            noise_key: x25519::PublicKey::from_base58_string(noise_key)
                .context("invalid noise_key")?,
            sphinx_key: x25519::PublicKey::from_base58_string(sphinx_key)
                .context("invalid sphinx_key")?,
            key_rotation_id: key_rotation_id as u32,
        })
    }

    /// The gateway probe's target: [`Self::mixnet_probe_target`] for the egress phase, plus the
    /// plain client websocket port the ingress phase opens its session on. The gateway role's
    /// eligibility requires that port, so its absence here is likewise a stored-data fault.
    pub(crate) fn gateway_probe_target(&self) -> anyhow::Result<api::GatewayProbeTarget> {
        let mixnet = self.mixnet_probe_target()?;
        let clients_ws_port = self
            .node
            .inner
            .clients_ws_port
            .context("missing clients_ws_port")?;

        Ok(api::GatewayProbeTarget {
            mixnet,
            clients_ws_port: u16::try_from(clients_ws_port)
                .context("clients_ws_port outside the port range")?,
        })
    }
}

/// Outcome of persisting a completed run: the id the run was stored under, and whether its
/// in-flight row was still there to clear. The submission path rejects a result whose lease has
/// already expired, so this is normally one - but the sweep can reap the row in the window between
/// that check and this insert, and the caller uses the count to keep the in-flight gauge honest.
pub(crate) struct InsertedTestRun {
    // no caller acts on the id yet - the submission path only needs to know whether a lock was
    // released - but an insert reporting what it stored is what the storage tests assert against
    #[allow(dead_code)]
    pub(crate) id: i64,
    pub(crate) cleared_in_progress: u64,
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

/// Convenience pass-through that delegates to the [`NewNymNode`] conversion.
impl TryFrom<NymNode> for NymNodeData {
    type Error = anyhow::Error;

    fn try_from(node: NymNode) -> anyhow::Result<Self> {
        node.inner.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn node(announced_ips: Option<&str>) -> NymNode {
        NymNode {
            inner: NewNymNode {
                node_id: 42,
                identity_key: "identity".to_string(),
                last_seen_bonded: datetime!(2026-08-01 00:00:00 UTC),
                mixnet_socket_address: Some("1.1.1.1:1789".to_string()),
                announced_ips: announced_ips.map(Into::into),
                noise_key: None,
                sphinx_key: None,
                key_rotation_id: None,
                node_type: NodeType::Mixnode,
                clients_ws_port: None,
            },
        }
    }

    #[test]
    fn consecutive_runs_rotate_through_every_announced_address() {
        let announced = node(Some("1.1.1.1,2.2.2.2,aaaa::1")).announced_ips();

        let mut tested = Vec::new();
        let mut previous = None;
        for _ in 0..4 {
            let next = next_ip_to_test(&announced, previous.as_deref()).unwrap();
            previous = Some(next.to_string());
            tested.push(next);
        }

        // every announced address gets exercised before the rotation wraps around
        assert_eq!(
            tested,
            vec![
                "1.1.1.1".parse::<IpAddr>().unwrap(),
                "2.2.2.2".parse().unwrap(),
                "aaaa::1".parse().unwrap(),
                "1.1.1.1".parse().unwrap(),
            ]
        );
    }

    #[test]
    fn rotation_restarts_when_the_pointer_is_no_longer_announced() {
        let announced = node(Some("1.1.1.1,2.2.2.2")).announced_ips();
        assert_eq!(
            next_ip_to_test(&announced, Some("9.9.9.9")),
            Some("1.1.1.1".parse::<IpAddr>().unwrap())
        );
    }

    // nodes that haven't been refreshed since `announced_ips` was introduced still have to be
    // testable, using whatever single address is on the row
    #[test]
    fn nodes_without_announced_ips_fall_back_to_the_stored_socket_address() {
        let announced = node(None).announced_ips();
        assert_eq!(announced, vec!["1.1.1.1".parse::<IpAddr>().unwrap()]);
        assert_eq!(
            next_ip_to_test(&announced, None),
            Some("1.1.1.1".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn malformed_announced_ips_are_skipped() {
        let announced = node(Some("not-an-ip,2.2.2.2")).announced_ips();
        assert_eq!(announced, vec!["2.2.2.2".parse::<IpAddr>().unwrap()]);
    }
}
