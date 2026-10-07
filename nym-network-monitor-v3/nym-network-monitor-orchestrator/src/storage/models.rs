// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use nym_api_requests::models::v3 as nym_api_requests;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::models::{
    self as api, InterfaceMeasurement, LatencyDistribution, RunMeasurements, TestRunData,
    TestRunInProgressData, TestRunResult,
};
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NymNodeBond;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use strum::{Display, EnumCount, EnumIter};
use time::OffsetDateTime;

pub(crate) fn duration_to_us(d: Duration) -> i64 {
    d.as_micros() as i64
}

pub(crate) fn us_to_duration(us: i64) -> Duration {
    Duration::from_micros(us as u64)
}

/// What a test run measures: one probe against one role of a node, which selects the run's cadence,
/// eligibility rules and expected measurement set. Like its API counterpart it deliberately has no
/// `Default`: a silently defaulted kind would measure the wrong thing rather than fail.
///
/// Each kind keeps its own work state in `node_test_state`, so a dual-role node is due separately
/// for the two liveness kinds and neither one's run moves the other's clock.
///
/// Every kind exists in order to be assigned, so the scheduler iterates the variants themselves
/// rather than a list kept in step with them by hand, and does so in DECLARATION ORDER, which puts
/// the liveness kinds first. The same holds for submission, where each kind is a stream of its own.
/// The spellings are the rows of the `test_kind` table every kind column references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, Display, EnumCount, EnumIter)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub(crate) enum TestKind {
    MixnodeLiveness,
    GatewayLiveness,
    MixnodeStress,
}

// The internal enum exists as a separate type so `sqlx::Type` can be derived without leaking sqlx
// into the public request crate; these conversions are the only bridge between the two.
impl From<TestKind> for api::TestKind {
    fn from(kind: TestKind) -> Self {
        match kind {
            TestKind::MixnodeLiveness => api::TestKind::MixnodeLiveness,
            TestKind::GatewayLiveness => api::TestKind::GatewayLiveness,
            TestKind::MixnodeStress => api::TestKind::MixnodeStress,
        }
    }
}

impl From<api::TestKind> for TestKind {
    fn from(kind: api::TestKind) -> Self {
        match kind {
            api::TestKind::MixnodeLiveness => TestKind::MixnodeLiveness,
            api::TestKind::GatewayLiveness => TestKind::GatewayLiveness,
            api::TestKind::MixnodeStress => TestKind::MixnodeStress,
        }
    }
}

/// The run-level columns every kind's results table shares, as written. Carries no `id`, which the
/// database assigns, and no measurements, which are the kind-shaped part of the row and decide which
/// table it goes in.
#[derive(Debug, Clone)]
pub(crate) struct NewTestRun {
    /// Contract-assigned node id of the node under test.
    pub(crate) node_id: i64,

    /// The address of that node that was tested, as reported by the agent that performed the run.
    pub(crate) tested_address: String,

    pub(crate) test_timestamp: OffsetDateTime,

    /// How long the test took, in microseconds. Run-level rather than per-measurement: a gateway
    /// run holds one session open across both of its phases, so the two cannot be timed apart.
    pub(crate) time_taken_us: i64,

    /// First error that caused the test to abort. `None` if the run completed without error.
    pub(crate) error: Option<String>,
}

impl NewTestRun {
    /// The run-level columns of a submitted result, recording the current UTC time as the test
    /// timestamp. The result's measurements are written alongside, into its kind's column groups.
    pub(crate) fn from_result(
        node_id: NodeId,
        tested_address: SocketAddr,
        result: &TestRunResult,
    ) -> Self {
        NewTestRun {
            node_id: node_id as i64,
            tested_address: tested_address.to_string(),
            test_timestamp: OffsetDateTime::now_utc(),
            time_taken_us: duration_to_us(result.time_taken),
            error: result.error.clone(),
        }
    }
}

/// The run-level columns of a row of any kind's results table, as read back.
#[derive(Debug, Clone)]
pub(crate) struct TestRun {
    pub(crate) id: i64,
    pub(crate) inner: NewTestRun,
}

/// A row of `mixnode_liveness_testrun` or `mixnode_stress_testrun`, whose columns are identical: the
/// run-level columns plus one `mix_forwarding` group. Flat because `query_as!` cannot flatten.
#[derive(Debug, Clone)]
pub(crate) struct MixnodeTestRunRow {
    pub(crate) id: i64,
    pub(crate) node_id: i64,
    pub(crate) tested_address: String,
    pub(crate) test_timestamp: OffsetDateTime,
    pub(crate) time_taken_us: i64,
    pub(crate) error: Option<String>,

    pub(crate) mix_forwarding_ingress_noise_handshake_us: Option<i64>,
    pub(crate) mix_forwarding_egress_noise_handshake_us: Option<i64>,
    pub(crate) mix_forwarding_sphinx_packet_delay_us: i64,
    pub(crate) mix_forwarding_packets_sent: i64,
    pub(crate) mix_forwarding_packets_received: i64,
    pub(crate) mix_forwarding_approximate_latency_us: Option<i64>,
    pub(crate) mix_forwarding_packets_rtt_min_us: Option<i64>,
    pub(crate) mix_forwarding_packets_rtt_mean_us: Option<i64>,
    pub(crate) mix_forwarding_packets_rtt_median_us: Option<i64>,
    pub(crate) mix_forwarding_packets_rtt_max_us: Option<i64>,
    pub(crate) mix_forwarding_packets_rtt_std_dev_us: Option<i64>,
    pub(crate) mix_forwarding_received_duplicates: bool,
}

impl MixnodeTestRunRow {
    /// The row read from `mixnode_liveness_testrun`.
    pub(crate) fn into_mixnode_liveness(self) -> CompletedTestRun {
        let mix_forwarding = self.mix_forwarding();
        CompletedTestRun {
            run: self.into_run(),
            measurements: RunMeasurements::MixnodeLiveness { mix_forwarding },
        }
    }

    /// The row read from `mixnode_stress_testrun`.
    pub(crate) fn into_mixnode_stress(self) -> CompletedTestRun {
        let mix_forwarding = self.mix_forwarding();
        CompletedTestRun {
            run: self.into_run(),
            measurements: RunMeasurements::MixnodeStress { mix_forwarding },
        }
    }

    fn mix_forwarding(&self) -> InterfaceMeasurement {
        InterfaceMeasurement {
            ingress_noise_handshake: self
                .mix_forwarding_ingress_noise_handshake_us
                .map(us_to_duration),
            egress_noise_handshake: self
                .mix_forwarding_egress_noise_handshake_us
                .map(us_to_duration),
            sphinx_packet_delay: us_to_duration(self.mix_forwarding_sphinx_packet_delay_us),
            packets_sent: self.mix_forwarding_packets_sent as usize,
            packets_received: self.mix_forwarding_packets_received as usize,
            approximate_latency: self
                .mix_forwarding_approximate_latency_us
                .map(us_to_duration),
            packets_statistics: latency_distribution(
                self.mix_forwarding_packets_rtt_min_us,
                self.mix_forwarding_packets_rtt_mean_us,
                self.mix_forwarding_packets_rtt_median_us,
                self.mix_forwarding_packets_rtt_max_us,
                self.mix_forwarding_packets_rtt_std_dev_us,
            ),
            received_duplicates: self.mix_forwarding_received_duplicates,
        }
    }

    fn into_run(self) -> TestRun {
        TestRun {
            id: self.id,
            inner: NewTestRun {
                node_id: self.node_id,
                tested_address: self.tested_address,
                test_timestamp: self.test_timestamp,
                time_taken_us: self.time_taken_us,
                error: self.error,
            },
        }
    }
}

/// A row of `gateway_liveness_testrun`: the run-level columns plus the `client_ingest` and
/// `client_delivery` groups. Flat because `query_as!` cannot flatten.
#[derive(Debug, Clone)]
pub(crate) struct GatewayLivenessTestRunRow {
    pub(crate) id: i64,
    pub(crate) node_id: i64,
    pub(crate) tested_address: String,
    pub(crate) test_timestamp: OffsetDateTime,
    pub(crate) time_taken_us: i64,
    pub(crate) error: Option<String>,

    pub(crate) client_ingest_ingress_noise_handshake_us: Option<i64>,
    pub(crate) client_ingest_egress_noise_handshake_us: Option<i64>,
    pub(crate) client_ingest_sphinx_packet_delay_us: i64,
    pub(crate) client_ingest_packets_sent: i64,
    pub(crate) client_ingest_packets_received: i64,
    pub(crate) client_ingest_approximate_latency_us: Option<i64>,
    pub(crate) client_ingest_packets_rtt_min_us: Option<i64>,
    pub(crate) client_ingest_packets_rtt_mean_us: Option<i64>,
    pub(crate) client_ingest_packets_rtt_median_us: Option<i64>,
    pub(crate) client_ingest_packets_rtt_max_us: Option<i64>,
    pub(crate) client_ingest_packets_rtt_std_dev_us: Option<i64>,
    pub(crate) client_ingest_received_duplicates: bool,

    pub(crate) client_delivery_ingress_noise_handshake_us: Option<i64>,
    pub(crate) client_delivery_egress_noise_handshake_us: Option<i64>,
    pub(crate) client_delivery_sphinx_packet_delay_us: i64,
    pub(crate) client_delivery_packets_sent: i64,
    pub(crate) client_delivery_packets_received: i64,
    pub(crate) client_delivery_approximate_latency_us: Option<i64>,
    pub(crate) client_delivery_packets_rtt_min_us: Option<i64>,
    pub(crate) client_delivery_packets_rtt_mean_us: Option<i64>,
    pub(crate) client_delivery_packets_rtt_median_us: Option<i64>,
    pub(crate) client_delivery_packets_rtt_max_us: Option<i64>,
    pub(crate) client_delivery_packets_rtt_std_dev_us: Option<i64>,
    pub(crate) client_delivery_received_duplicates: bool,
}

impl GatewayLivenessTestRunRow {
    pub(crate) fn into_gateway_liveness(self) -> CompletedTestRun {
        let client_ingest = self.client_ingest();
        let client_delivery = self.client_delivery();
        CompletedTestRun {
            run: self.into_run(),
            measurements: RunMeasurements::GatewayLiveness {
                client_ingest,
                client_delivery,
            },
        }
    }

    fn client_ingest(&self) -> InterfaceMeasurement {
        InterfaceMeasurement {
            ingress_noise_handshake: self
                .client_ingest_ingress_noise_handshake_us
                .map(us_to_duration),
            egress_noise_handshake: self
                .client_ingest_egress_noise_handshake_us
                .map(us_to_duration),
            sphinx_packet_delay: us_to_duration(self.client_ingest_sphinx_packet_delay_us),
            packets_sent: self.client_ingest_packets_sent as usize,
            packets_received: self.client_ingest_packets_received as usize,
            approximate_latency: self
                .client_ingest_approximate_latency_us
                .map(us_to_duration),
            packets_statistics: latency_distribution(
                self.client_ingest_packets_rtt_min_us,
                self.client_ingest_packets_rtt_mean_us,
                self.client_ingest_packets_rtt_median_us,
                self.client_ingest_packets_rtt_max_us,
                self.client_ingest_packets_rtt_std_dev_us,
            ),
            received_duplicates: self.client_ingest_received_duplicates,
        }
    }

    fn client_delivery(&self) -> InterfaceMeasurement {
        InterfaceMeasurement {
            ingress_noise_handshake: self
                .client_delivery_ingress_noise_handshake_us
                .map(us_to_duration),
            egress_noise_handshake: self
                .client_delivery_egress_noise_handshake_us
                .map(us_to_duration),
            sphinx_packet_delay: us_to_duration(self.client_delivery_sphinx_packet_delay_us),
            packets_sent: self.client_delivery_packets_sent as usize,
            packets_received: self.client_delivery_packets_received as usize,
            approximate_latency: self
                .client_delivery_approximate_latency_us
                .map(us_to_duration),
            packets_statistics: latency_distribution(
                self.client_delivery_packets_rtt_min_us,
                self.client_delivery_packets_rtt_mean_us,
                self.client_delivery_packets_rtt_median_us,
                self.client_delivery_packets_rtt_max_us,
                self.client_delivery_packets_rtt_std_dev_us,
            ),
            received_duplicates: self.client_delivery_received_duplicates,
        }
    }

    fn into_run(self) -> TestRun {
        TestRun {
            id: self.id,
            inner: NewTestRun {
                node_id: self.node_id,
                tested_address: self.tested_address,
                test_timestamp: self.test_timestamp,
                time_taken_us: self.time_taken_us,
                error: self.error,
            },
        }
    }
}

/// Reassembles a [`LatencyDistribution`] from its five flattened microsecond columns.
/// Returns `None` if any column is `NULL`; the five columns are always written all-set or all-NULL
/// together.
pub(crate) fn latency_distribution(
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

/// A completed run as stored: its run-level columns plus what it measured, shaped by its kind.
/// This is the unit both the operator read surface and the nym-api submission path consume, since
/// a run's score is defined over its whole measurement set.
#[derive(Debug, Clone)]
pub(crate) struct CompletedTestRun {
    pub(crate) run: TestRun,
    pub(crate) measurements: RunMeasurements,
}

impl CompletedTestRun {
    /// The run's score: the delivery score averaged over every interface its kind exercises.
    ///
    /// The average is taken over the interfaces the kind DEFINES rather than over the ones that
    /// came back, so a phase that produced nothing scores zero instead of shrinking the denominator:
    /// a gateway whose delivery never ran must not tie with one that passed both. That set is the
    /// shape of the measurements themselves, so no interface can be missing from it. The result is
    /// already normalised into `[0.0, 1.0]` and so comparable across kinds.
    pub(crate) fn score(&self) -> f64 {
        let measurements = self.measurements.all();
        let total: f64 = measurements
            .iter()
            .map(|measurement| delivery_score(measurement))
            .sum();
        total / measurements.len() as f64
    }
}

/// One interface's delivery ratio.
///
/// A measurement that saw duplicates is discarded whole rather than scored, because an honest node
/// never replays a packet and the ratio alone cannot tell the two apart: a node that forwards one
/// packet and echoes it nine more times counts ten received against ten sent and would otherwise
/// score a perfect 1.0 for having delivered a tenth of the traffic.
fn delivery_score(measurement: &InterfaceMeasurement) -> f64 {
    if measurement.received_duplicates {
        return 0.0;
    }
    // the ratio (and its clamp) is defined once, on the API-level measurement
    measurement.received_ratio()
}

/// Lifts a completed run into the public [`TestRunData`] shape: widens `i64` ids to the API's
/// `u32` and converts the run's microsecond duration back into a `std::time::Duration`.
impl From<CompletedTestRun> for TestRunData {
    fn from(completed: CompletedTestRun) -> Self {
        let run = completed.run;
        let inner = run.inner;

        TestRunData {
            id: run.id,
            node_id: inner.node_id as u32,
            // a malformed stored address is not worth failing the whole result over,
            // it's informational rather than something we act on
            tested_address: inner.tested_address.parse().ok(),
            test_timestamp: inner.test_timestamp,
            result: TestRunResult {
                time_taken: us_to_duration(inner.time_taken_us),
                error: inner.error,
                measurements: completed.measurements,
            },
        }
    }
}

/// Projects a completed stress run onto the nym-api's `StressTestResult` shape used by the
/// stress-test batch submission endpoint.
///
/// Two fields are synthesised here rather than stored directly:
///
/// - `test_performance` is the run's [`score`](CompletedTestRun::score), i.e. the delivery ratio of
///   its one `mix_forwarding` measurement. A run that sent no packets or saw duplicates collapses
///   to `0.0`; `was_reachable` is what lets the server tell that apart from a genuine zero score.
/// - `was_reachable` is `error.is_none()` — i.e. the test completed without an abort error. A run
///   that aborted before the node responded sets `error` to the first failure, so the inverse is
///   an accurate "did we reach the node at all" signal.
impl From<&CompletedTestRun> for nym_api_requests::StressTestResult {
    fn from(completed: &CompletedTestRun) -> Self {
        let inner = &completed.run.inner;

        nym_api_requests::StressTestResult {
            testrun_id: completed.run.id,
            node_id: inner.node_id as u32,
            // the stress stream carries `mixnode_stress` runs only, which probe a mixing hop by
            // definition
            is_mixnode: true,
            test_timestamp: inner.test_timestamp,
            test_performance: completed.score(),
            was_reachable: inner.error.is_none(),
        }
    }
}

/// Projects a completed liveness run onto the nym-api's `LivenessTestResult` shape: a single
/// score, identical for both liveness kinds.
///
/// The averaging behind [`score`](CompletedTestRun::score) happens HERE because the submission
/// carries neither the kind nor the interfaces, so nym-api could not reconstruct its denominator.
/// The per-interface breakdown stays in local storage under the run's row, where the operator read
/// surface serves it. Submitting it would put figures on the wire that nym-api does not score, and
/// those would have to be versioned before they could be trusted - the same call made for the
/// latency distribution. `was_reachable` is `error.is_none()`, as on the stress path.
impl From<&CompletedTestRun> for nym_api_requests::LivenessTestResult {
    fn from(completed: &CompletedTestRun) -> Self {
        let inner = &completed.run.inner;

        nym_api_requests::LivenessTestResult {
            testrun_id: completed.run.id,
            node_id: inner.node_id as u32,
            test_timestamp: inner.test_timestamp,
            test_performance: completed.score(),
            was_reachable: inner.error.is_none(),
        }
    }
}

/// What the mixnet contract says about a node: a row of `nym_node_bond`. Written for every bonded
/// node on every refresh, whether or not the node's own endpoint answered.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct BondedNymNode {
    /// Node ID as assigned by the mixnet contract.
    pub(crate) node_id: i64,

    /// Ed25519 identity key, base58-encoded.
    /// A node_id always maps to exactly one identity_key and is never reassigned.
    pub(crate) identity_key: String,

    /// When this node was last observed as bonded in the contract.
    pub(crate) last_seen_bonded: OffsetDateTime,
}

impl BondedNymNode {
    /// The bond as the refresh that read the contract at `seen_at` found it. One timestamp per
    /// refresh, shared by every bond it read, which is what lets the nodes it did NOT see be
    /// recognised afterwards by an older timestamp.
    pub(crate) fn from_bond(bond: &NymNodeBond, seen_at: OffsetDateTime) -> Self {
        BondedNymNode {
            node_id: bond.node_id as i64,
            identity_key: bond.identity().to_string(),
            last_seen_bonded: seen_at,
        }
    }
}

/// What a node's own endpoint reported about it: a row of `nym_node_description`, only ever
/// written from a complete reading. Carries no test state: staleness and the rotation pointer live
/// in [`NodeTestState`], keyed per kind.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NodeDescription {
    pub(crate) node_id: i64,

    /// Port of the node's mixnet listener. The address under test comes from the rotation over the
    /// announced set rather than being stored beside the port.
    pub(crate) mix_port: i64,

    /// Every ip address the node announced, comma-separated. Canonicalised, deduplicated and sorted
    /// on write, which is what keeps the [`next_ip_to_test`] rotation stable across refreshes.
    /// Never empty: a node announcing no address cannot be described.
    pub(crate) announced_ips: String,

    /// X25519 public key used for Noise handshakes, base58-encoded.
    pub(crate) noise_key: String,

    /// Sphinx public key used for packet encryption, base58-encoded, and the key rotation epoch it
    /// belongs to.
    pub(crate) sphinx_key: String,
    pub(crate) key_rotation_id: i64,

    /// The roles the node reports. `gateway_enabled` is the entry-gateway role.
    pub(crate) mixnode_enabled: bool,
    pub(crate) gateway_enabled: bool,

    /// Port of the node's PLAIN client websocket listener, which a gateway liveness probe opens its
    /// session on. Present exactly when `gateway_enabled` is set.
    pub(crate) clients_ws_port: Option<i64>,
}

/// A node as the registry holds it: always its bond, and its description only when the node
/// answered completely, so a partly described node is unrepresentable. What one refresh writes and
/// what the read surface serves.
pub(crate) struct NymNode {
    pub(crate) bond: BondedNymNode,
    pub(crate) description: Option<NodeDescription>,
}

/// Lifts a stored node into the public shape, decoding its base58 keys and comma-separated
/// addresses. The orchestrator writes every one of these itself, so a failure means corruption.
impl TryFrom<NymNode> for api::NymNodeData {
    type Error = anyhow::Error;

    fn try_from(node: NymNode) -> Result<Self, Self::Error> {
        let bond = node.bond;

        Ok(api::NymNodeData {
            node_id: bond.node_id as u32,
            identity_key: ed25519::PublicKey::from_base58_string(&bond.identity_key)
                .context("invalid identity_key")?,
            last_seen_bonded: bond.last_seen_bonded,
            description: node.description.map(TryInto::try_into).transpose()?,
        })
    }
}

impl TryFrom<NodeDescription> for api::NymNodeDescriptionData {
    type Error = anyhow::Error;

    fn try_from(description: NodeDescription) -> Result<Self, Self::Error> {
        let announced_ips = description
            .announced_ips
            .split(',')
            .map(|ip| ip.parse::<IpAddr>())
            .collect::<Result<_, _>>()
            .context("invalid announced_ips")?;
        let clients_ws_port = description
            .clients_ws_port
            .map(u16::try_from)
            .transpose()
            .context("clients_ws_port outside the port range")?;

        Ok(api::NymNodeDescriptionData {
            mix_port: u16::try_from(description.mix_port)
                .context("mix_port outside the port range")?,
            announced_ips,
            noise_key: x25519::PublicKey::from_base58_string(&description.noise_key)
                .context("invalid noise_key")?,
            sphinx_key: x25519::PublicKey::from_base58_string(&description.sphinx_key)
                .context("invalid sphinx_key")?,
            key_rotation_id: u32::try_from(description.key_rotation_id)
                .context("key_rotation_id outside the u32 range")?,
            mixnode_enabled: description.mixnode_enabled,
            gateway_enabled: description.gateway_enabled,
            clients_ws_port,
        })
    }
}

/// The ip a given kind should test next: the one following `previously_tested_ip` in `announced`,
/// so consecutive runs of that kind rotate through every address the node has. Falls back to the
/// first announced address when the pointer is unset (the kind has never assigned this node) or no
/// longer announced.
///
/// The announced set belongs to the node while the pointer belongs to the kind, which is what lets
/// two kinds advance over the same set independently instead of skipping addresses because of each
/// other.
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

/// The refresh time every fixture node's bond carries, so seeding fixtures through
/// `store_refresh` at this time never strips another fixture's description.
#[cfg(test)]
pub(crate) const FIXTURE_SEEN_AT: OffsetDateTime = time::macros::datetime!(2025-01-01 00:00:00 UTC);

/// A run against `node_id`, i.e. the baseline a test overrides only the fields it is actually
/// asserting on.
#[cfg(test)]
pub(crate) fn minimal_test_run(node_id: i64) -> NewTestRun {
    NewTestRun {
        node_id,
        tested_address: "1.2.3.4:1789".to_string(),
        test_timestamp: time::macros::datetime!(2025-06-01 12:00:00 UTC),
        time_taken_us: 0,
        error: None,
    }
}

/// A bonded node described with the given roles and announcing `announced_ips` (comma-separated).
/// Its keys are real, seeded by `node_id`, so its probe targets decode; a gateway gets the client
/// websocket port its description cannot be stored without.
#[cfg(test)]
pub(crate) fn described_node(
    node_id: i64,
    announced_ips: &str,
    mixnode_enabled: bool,
    gateway_enabled: bool,
) -> NymNode {
    use nym_test_utils::helpers::seeded_rng;

    let seed = [node_id as u8; 32];
    let x25519_key = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut seeded_rng(seed)));
    let identity_key = *ed25519::KeyPair::new(&mut seeded_rng(seed)).public_key();

    NymNode {
        bond: BondedNymNode {
            node_id,
            identity_key: identity_key.to_base58_string(),
            last_seen_bonded: FIXTURE_SEEN_AT,
        },
        description: Some(NodeDescription {
            node_id,
            mix_port: 1789,
            announced_ips: announced_ips.to_string(),
            noise_key: x25519_key.to_base58_string(),
            sphinx_key: x25519_key.to_base58_string(),
            key_rotation_id: 7,
            mixnode_enabled,
            gateway_enabled,
            clients_ws_port: gateway_enabled.then_some(9000),
        }),
    }
}

/// A mixnode announcing `1.2.3.4`.
#[cfg(test)]
pub(crate) fn mixnode(node_id: i64) -> NymNode {
    described_node(node_id, "1.2.3.4", true, false)
}

/// A gateway (and nothing else) announcing `1.2.3.4`, with client websocket port 9000.
#[cfg(test)]
pub(crate) fn gateway(node_id: i64) -> NymNode {
    described_node(node_id, "1.2.3.4", false, true)
}

/// A measurement with every optional figure unset and no packets sent, i.e. the baseline a test
/// overrides only the fields it is actually asserting on.
#[cfg(test)]
pub(crate) fn minimal_measurement() -> InterfaceMeasurement {
    InterfaceMeasurement {
        ingress_noise_handshake: None,
        egress_noise_handshake: None,
        sphinx_packet_delay: Duration::ZERO,
        packets_sent: 0,
        packets_received: 0,
        approximate_latency: None,
        packets_statistics: None,
        received_duplicates: false,
    }
}

/// Every interface `kind` exercises, each at [`minimal_measurement`].
#[cfg(test)]
pub(crate) fn minimal_measurements(kind: TestKind) -> RunMeasurements {
    match kind {
        TestKind::MixnodeLiveness => RunMeasurements::MixnodeLiveness {
            mix_forwarding: minimal_measurement(),
        },
        TestKind::GatewayLiveness => RunMeasurements::GatewayLiveness {
            client_ingest: minimal_measurement(),
            client_delivery: minimal_measurement(),
        },
        TestKind::MixnodeStress => RunMeasurements::MixnodeStress {
            mix_forwarding: minimal_measurement(),
        },
    }
}

/// A row from the `node_test_state` table, less the node id its readers already filter on: what one
/// kind has done against one node so far. Only tests read a whole row; production code writes its
/// columns individually.
///
/// Every column beyond the key is nullable because a row is created by whichever path touches the
/// kind first - the assignment writes only [`Self::last_tested_ip`], the result submission only
/// [`Self::last_tested_at`].
#[cfg(test)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct NodeTestState {
    pub(crate) test_kind: TestKind,

    /// When this kind last completed a run against the node, which is what the staleness gate
    /// reads. `None` while the node has only ever been assigned, never measured. Stored directly
    /// rather than derived from the kind's results so that evicting an old result does not make the
    /// node read as never-tested and jump the assignment queue.
    pub(crate) last_tested_at: Option<OffsetDateTime>,

    /// The address handed out for this kind's most recent assignment, i.e. its rotation pointer
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
    pub(crate) expires_at: OffsetDateTime,

    /// What the run was dispatched to measure. This is the authoritative source of the kind when
    /// the result comes back: the submission reports only the node and the address, so reading it
    /// from here is what keeps the orchestrator from trusting an agent's echo of a value the
    /// orchestrator itself chose.
    pub(crate) test_kind: TestKind,
}

/// Lifts a `testrun_in_progress` row into the public shape, narrowing `node_id` from the
/// sqlx-native `i64` to the API's `u32`. The lease and kind come across as stored: they are what
/// the orchestrator chose at dispatch, so the read surface shows what an agent was actually asked
/// for rather than what it later claims to have run.
impl From<TestRunInProgress> for TestRunInProgressData {
    fn from(row: TestRunInProgress) -> Self {
        TestRunInProgressData {
            node_id: row.node_id as u32,
            test_kind: row.test_kind.into(),
            started_at: row.started_at,
            expires_at: row.expires_at,
        }
    }
}

/// A node one kind could assign right now: what its probe target is built from, joined onto that
/// kind's rotation pointer and staleness position. Only those two are taken from the state side,
/// which is what keeps each kind's rotation and staleness independent of every other.
#[derive(Debug, Clone)]
pub(crate) struct AssignmentCandidate {
    pub(crate) node_id: i64,

    /// Ed25519 identity key, base58-encoded, from the node's bond.
    pub(crate) identity_key: String,

    /// The rest come from the node's description; see [`NodeDescription`].
    pub(crate) mix_port: i64,
    pub(crate) announced_ips: String,
    pub(crate) noise_key: String,
    pub(crate) sphinx_key: String,
    pub(crate) key_rotation_id: i64,
    pub(crate) clients_ws_port: Option<i64>,

    /// The address this kind handed out last time, i.e. its rotation pointer into the announced set.
    pub(crate) last_tested_ip: Option<String>,

    /// When this kind last measured the node, or `None` if it never has.
    pub(crate) last_tested_at: Option<OffsetDateTime>,
}

impl AssignmentCandidate {
    /// Every ip address the node announced. An unparseable entry is skipped rather than failing the
    /// whole assignment.
    pub(crate) fn announced_ips(&self) -> Vec<IpAddr> {
        self.announced_ips
            .split(',')
            .filter_map(|ip| ip.trim().parse().ok())
            .collect()
    }
}

/// When the node a kind would assign next fell due, which is what the scheduler compares across
/// kinds.
///
/// A DUE time rather than a last-tested time, because kinds run at different cadences: a node a
/// two-hour kind last tested long ago may be due later than one a fifteen-minute kind tested
/// recently. `Ord` comes from the declaration order and then from the timestamp, so `NeverTested`
/// outranks every measured node and an earlier due time outranks a later one, which makes the most
/// overdue head the minimum.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum KindHead {
    NeverTested,
    DueAt(OffsetDateTime),
}

/// How one kind is scheduled, as its configuration expresses it. Resolved into an
/// [`AssignmentRequest`] against a single `now`.
#[derive(Debug, Copy, Clone)]
pub(crate) struct KindSchedule {
    pub(crate) kind: TestKind,

    /// Minimum time since this kind's last run against a node before it is due again.
    pub(crate) staleness_age: Duration,

    /// How long a dispatched run holds its node before the lease expires.
    pub(crate) lease_budget: Duration,

    /// Upper bound on the targets one assignment may carry: one for a stress run, the kind's wave
    /// size for a liveness run.
    pub(crate) wave_size: usize,
}

/// One kind's dispatch parameters with every duration already resolved against one `now`, which
/// is what the assignment query binds. Absolute rather than relative so a caller can hold a single
/// timestamp across the gates it applies and the rows it stamps.
#[derive(Debug, Copy, Clone)]
pub(crate) struct AssignmentRequest {
    pub(crate) kind: TestKind,

    /// Stamped as `started_at` on every in-progress row this assignment writes.
    pub(crate) now: OffsetDateTime,

    /// Staleness gate: a node this kind has tested before is eligible only if that run predates
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
    pub(crate) node: AssignmentCandidate,

    /// The announced ip picked for this run by [`next_ip_to_test`].
    pub(crate) tested_ip: IpAddr,
}

impl AssignedTestrun {
    /// The target an agent probes over the node's mixnet listener: the stored keys decoded, and the
    /// address this run rotated onto carrying the node's announced mix port.
    ///
    /// The orchestrator writes every one of these fields itself, so a value that will not decode
    /// means corruption or a schema regression rather than an untestable node.
    pub(crate) fn mixnet_probe_target(&self) -> anyhow::Result<api::MixnetProbeTarget> {
        let node = &self.node;

        Ok(api::MixnetProbeTarget {
            node_id: node.node_id as u32,
            identity_key: ed25519::PublicKey::from_base58_string(&node.identity_key)
                .context("invalid identity_key")?,
            node_address: SocketAddr::new(
                self.tested_ip,
                u16::try_from(node.mix_port).context("mix_port outside the port range")?,
            ),
            node_ips: node.announced_ips(),
            noise_key: x25519::PublicKey::from_base58_string(&node.noise_key)
                .context("invalid noise_key")?,
            sphinx_key: x25519::PublicKey::from_base58_string(&node.sphinx_key)
                .context("invalid sphinx_key")?,
            key_rotation_id: node.key_rotation_id as u32,
        })
    }

    /// The gateway probe's target: [`Self::mixnet_probe_target`] for the egress phase, plus the
    /// plain client websocket port the ingress phase opens its session on. A gateway description
    /// always carries that port, so its absence here is likewise a stored-data fault.
    pub(crate) fn gateway_probe_target(&self) -> anyhow::Result<api::GatewayProbeTarget> {
        let mixnet = self.mixnet_probe_target()?;
        let clients_ws_port = self
            .node
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

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn candidate(announced_ips: &str) -> AssignmentCandidate {
        AssignmentCandidate {
            node_id: 42,
            identity_key: "identity".to_string(),
            mix_port: 1789,
            announced_ips: announced_ips.to_string(),
            noise_key: String::new(),
            sphinx_key: String::new(),
            key_rotation_id: 0,
            clients_ws_port: None,
            last_tested_ip: None,
            last_tested_at: None,
        }
    }

    #[test]
    fn consecutive_runs_rotate_through_every_announced_address() {
        let announced = candidate("1.1.1.1,2.2.2.2,aaaa::1").announced_ips();

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
        let announced = candidate("1.1.1.1,2.2.2.2").announced_ips();
        assert_eq!(
            next_ip_to_test(&announced, Some("9.9.9.9")),
            Some("1.1.1.1".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn malformed_announced_ips_are_skipped() {
        let announced = candidate("not-an-ip,2.2.2.2").announced_ips();
        assert_eq!(announced, vec!["2.2.2.2".parse::<IpAddr>().unwrap()]);
    }

    /// The lease and the kind are what let an operator tell a slow run from an abandoned one, and
    /// a dual-role node's two concurrent-looking runs from each other. Distinct timestamps because
    /// `started_at` and `expires_at` share a type and would transpose silently.
    #[test]
    fn an_in_progress_run_carries_its_kind_and_lease() {
        let row = TestRunInProgress {
            node_id: 42,
            started_at: datetime!(2026-08-01 00:00:00 UTC),
            expires_at: datetime!(2026-08-01 00:01:00 UTC),
            test_kind: TestKind::GatewayLiveness,
        };

        let data = TestRunInProgressData::from(row);

        assert_eq!(data.node_id, 42);
        assert_eq!(data.test_kind, api::TestKind::GatewayLiveness);
        assert_eq!(data.started_at, datetime!(2026-08-01 00:00:00 UTC));
        assert_eq!(data.expires_at, datetime!(2026-08-01 00:01:00 UTC));
    }

    /// The scoring of a liveness run, i.e. what the nym-api is asked to weight a node by.
    mod liveness_submission {
        use super::*;

        /// A measurement that received `received` of the `sent` packets it was given.
        fn measurement(sent: usize, received: usize) -> InterfaceMeasurement {
            InterfaceMeasurement {
                packets_sent: sent,
                packets_received: received,
                ..minimal_measurement()
            }
        }

        fn liveness_run(measurements: RunMeasurements) -> CompletedTestRun {
            CompletedTestRun {
                run: TestRun {
                    id: 7,
                    inner: NewTestRun {
                        node_id: 42,
                        tested_address: "1.1.1.1:1789".to_string(),
                        test_timestamp: datetime!(2026-08-01 00:00:00 UTC),
                        time_taken_us: 0,
                        error: None,
                    },
                },
                measurements,
            }
        }

        #[test]
        fn a_mixnode_run_scores_its_single_interface() {
            let run = liveness_run(RunMeasurements::MixnodeLiveness {
                mix_forwarding: measurement(10, 5),
            });

            let result = nym_api_requests::LivenessTestResult::from(&run);

            assert_eq!(result.test_performance, 0.5);
        }

        #[test]
        fn a_gateway_run_averages_both_of_its_phases() {
            let run = liveness_run(RunMeasurements::GatewayLiveness {
                client_ingest: measurement(10, 10),
                client_delivery: measurement(10, 5),
            });

            let result = nym_api_requests::LivenessTestResult::from(&run);

            // (1.0 + 0.5) / 2 - the two phases are not distinguishable in the submitted score,
            // only in the measurements kept locally
            assert_eq!(result.test_performance, 0.75);
        }

        /// The point of the fixed denominator: a phase that sent nothing must still count, or a
        /// gateway whose delivery never ran would tie with one that passed both phases.
        #[test]
        fn an_unmeasured_phase_scores_zero_rather_than_shrinking_the_denominator() {
            let run = liveness_run(RunMeasurements::GatewayLiveness {
                client_ingest: measurement(10, 10),
                client_delivery: minimal_measurement(),
            });

            let result = nym_api_requests::LivenessTestResult::from(&run);

            // 1.0 / 2, not 1.0 / 1: the unmeasured phase is still in the denominator
            assert_eq!(result.test_performance, 0.5);
        }

        /// A node that forwards one packet and replays it nine more times counts ten received
        /// against ten sent, so the ratio alone would hand it a perfect score for delivering a
        /// tenth of the traffic.
        #[test]
        fn an_interface_that_saw_duplicates_scores_zero() {
            let run = liveness_run(RunMeasurements::GatewayLiveness {
                client_ingest: InterfaceMeasurement {
                    received_duplicates: true,
                    ..measurement(10, 10)
                },
                client_delivery: measurement(10, 10),
            });

            let result = nym_api_requests::LivenessTestResult::from(&run);

            // zeroing is scoped to the interface that replayed, not to the whole run: the healthy
            // delivery phase still contributes its 1.0
            assert_eq!(result.test_performance, 0.5);
        }
    }
}
