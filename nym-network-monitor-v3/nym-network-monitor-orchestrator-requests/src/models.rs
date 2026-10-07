// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_crypto::asymmetric::x25519;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use strum::Display;
use time::OffsetDateTime;

/// The pair of mixnet addresses announced by an agent. Depending on the family a tested node was
/// reached over, it sees one or the other as the source of the test traffic, so both are authorised
/// in the network monitors contract.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct AgentMixAddresses {
    /// V4 egress address of the agent node
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub v4: SocketAddr,

    /// V6 egress address of the agent node
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub v6: SocketAddr,
}

impl AgentMixAddresses {
    /// Whether the two addresses are actually one address of each family. `v6` must not hold an
    /// ipv4-mapped address either: nodes canonicalise the authorised agent addresses, so such an
    /// entry would collapse onto the ipv4 one and leave the agent with a single authorised ingress
    /// while both the contract and this orchestrator believe it has two.
    pub fn has_distinct_families(&self) -> bool {
        self.v4.is_ipv4() && self.v6.ip().to_canonical().is_ipv6()
    }

    /// The address a node at `tested_node_address` should send the test packets back to, i.e. the
    /// one of the same family, so that a test run exercises a single family in both directions.
    pub fn matching_family(&self, tested_node_address: SocketAddr) -> SocketAddr {
        if tested_node_address.ip().to_canonical().is_ipv6() {
            self.v6
        } else {
            self.v4
        }
    }
}

#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Body sent by an agent to announce its details to the orchestrator.
/// The orchestrator forwards this information to the smart contract so that
/// network nodes can whitelist connections from known agents.
pub struct AgentAnnounceRequest {
    /// Egress addresses of the agent node
    pub mix_addresses: AgentMixAddresses,

    /// Base-58 encoded noise key of the agent.
    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub x25519_noise_key: x25519::PublicKey,

    /// Version of the noise protocol used by the agent.
    pub noise_version: u8,

    /// Base-58 encoded ed25519 identity the agent presents when opening a gateway client session.
    #[serde(with = "bs58_ed25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub ed25519_identity: ed25519::PublicKey,
}

/// Confirmation returned to an agent after a successful announcement.
/// Currently empty — exists to give the response an explicit type rather than
/// relying on `Json(())`.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnnounceResponse {}

/// Request sent by an agent to ask the orchestrator for a node to test.
/// Identifies the agent so the orchestrator can verify it has been announced.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunAssignmentRequest {
    /// Egress addresses of the agent node
    pub mix_addresses: AgentMixAddresses,

    /// Base-58 encoded noise key of the agent.
    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub x25519_noise_key: x25519::PublicKey,
}

/// What a test run measures: one probe against one role of a node, so a dual-role node is due
/// separately for each of the two liveness kinds.
///
/// Deliberately has no `Default` - the kind decides eligibility, cadence and the expected
/// measurement set, so a silently defaulted value would measure the wrong thing rather than fail.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TestKind {
    /// Low-volume delivery-ratio probe of a node's mix forwarding, a wave of targets per assignment.
    MixnodeLiveness,

    /// Two-phase delivery-ratio probe of a gateway's client ingest and delivery, a wave of targets
    /// per assignment.
    GatewayLiveness,

    /// High-volume throughput probe of a node's mix forwarding, one target per assignment.
    MixnodeStress,
}

/// Response from the orchestrator when an agent requests work.
/// `assignment` is `None` when no nodes are due for testing.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunAssignmentResponse {
    pub assignment: Option<TestRunAssignment>,
}

/// Work handed to an agent, one variant per [`TestKind`].
///
/// `MixnodeStress` and `MixnodeLiveness` carry the same payload because they are the same probe,
/// differing only in the profile the agent applies, which the agent holds in its own config and
/// selects from the tag. `GatewayLiveness` additionally carries what is needed to open a client
/// websocket session.
///
/// A stress assignment is ONE target; a liveness assignment is a WAVE the agent probes
/// concurrently, so the lease the orchestrator stamps is bounded by the slowest single target
/// rather than by their sum. A wave holds targets of one kind only, so a dual-role node is
/// assigned each of its liveness kinds separately.
///
/// An assignment with no targets is NOT a valid assignment. "No work" is expressed by an absent
/// assignment on [`TestRunAssignmentResponse`], so the orchestrator must not emit an empty wave.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRunAssignment {
    MixnodeStress(Box<MixnetProbeTarget>),
    MixnodeLiveness(Vec<MixnetProbeTarget>),
    GatewayLiveness(Vec<GatewayProbeTarget>),
}

impl TestRunAssignment {
    /// What this assignment measures. Determines the profile the agent applies, and the kind
    /// recorded against the resulting run.
    pub fn kind(&self) -> TestKind {
        match self {
            TestRunAssignment::MixnodeLiveness(_) => TestKind::MixnodeLiveness,
            TestRunAssignment::GatewayLiveness(_) => TestKind::GatewayLiveness,
            TestRunAssignment::MixnodeStress(_) => TestKind::MixnodeStress,
        }
    }
}

/// A node to probe over its mixnet listener.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixnetProbeTarget {
    pub node_id: u32,

    /// The node's ed25519 identity, as bonded in the mixnet contract. Every bonded node has one,
    /// so it is always available regardless of what else the orchestrator has learned about the
    /// node. Carried on every target rather than only where a probe consumes it today: the gateway
    /// probe authenticates the node with it during the client registration handshake, and it is the
    /// key any future signature check over a node's responses would verify against.
    #[serde(with = "bs58_ed25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub identity_key: ed25519::PublicKey,

    /// The address of the node that should be tested, i.e. the one the agent is expected to send
    /// the test packets to. Always one of [`Self::node_ips`] combined with the node's mix port.
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub node_address: SocketAddr,

    /// Every ip address the node has announced. The node isn't guaranteed to send the test packets
    /// back from the address it was reached on (it may be multi-homed, or reached over a different
    /// family than it replies over), so the agent has to accept a return connection from any of
    /// them.
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub node_ips: Vec<IpAddr>,

    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub noise_key: x25519::PublicKey,

    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub sphinx_key: x25519::PublicKey,

    pub key_rotation_id: u32,
}

/// A node to probe as an entry gateway: its mixnet listener for the egress phase, plus the client
/// websocket details for the ingress phase.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayProbeTarget {
    pub mixnet: MixnetProbeTarget,

    /// Port of the node's PLAIN client websocket listener. The session is established against
    /// `ws://<one of the mixnet target's ips>:<this port>`, never an announced hostname or a wss
    /// entry, so that no proxy sits between the agent and the gateway. The identity the handshake
    /// authenticates the gateway against is [`MixnetProbeTarget::identity_key`].
    pub clients_ws_port: u16,
}

/// Round-trip time statistics computed over the test packets received during a run.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatencyDistribution {
    /// Minimum round-trip time of a test packet.
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub minimum: Duration,

    /// Average round-trip time of a test packet.
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub mean: Duration,

    /// Median round-trip time of a test packet.
    /// For an even number of samples, this is the arithmetic mean of the two middle values.
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub median: Duration,

    /// Maximum round-trip time of a test packet.
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub maximum: Duration,

    /// The standard deviation of the test packets' round-trip times.
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub standard_deviation: Duration,
}

/// Request sent by an agent to submit test results for a previously assigned node.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResultSubmissionRequest {
    pub node_id: u32,

    /// The address that was actually tested. A node may announce several addresses and only some
    /// of them may be healthy, so the result is meaningless without knowing which one it refers to.
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub tested_address: SocketAddr,

    pub result: TestRunResult,
}

/// What one run measured, shaped by its kind: one measurement per interface the kind exercises.
///
/// The variant IS the kind, and its fields ARE the interfaces it is expected to produce, so a result
/// can neither omit an interface its kind requires nor carry one it does not, and its kind cannot
/// disagree with what it measured. Each interface is named for the node FUNCTION under measurement
/// rather than a route, because every one traverses the mixnet in some form. A gateway's two are
/// kept apart because averaging them at the agent would make a healthy ingest with a dead delivery
/// indistinguishable from a uniformly half-lossy node.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMeasurements {
    /// The node forwarding as a mixing hop, measured by the two-hop self-loop through its mixnet
    /// listener.
    MixnodeLiveness {
        mix_forwarding: InterfaceMeasurement,
    },

    /// The node accepting packets from a client session and injecting them into the mixnet, and
    /// taking final-hop packets off the mixnet and delivering them to a client session.
    GatewayLiveness {
        client_ingest: InterfaceMeasurement,
        client_delivery: InterfaceMeasurement,
    },

    /// The node forwarding as a mixing hop under load.
    MixnodeStress {
        mix_forwarding: InterfaceMeasurement,
    },
}

impl RunMeasurements {
    /// The kind these measurements were taken for.
    pub fn kind(&self) -> TestKind {
        match self {
            RunMeasurements::MixnodeLiveness { .. } => TestKind::MixnodeLiveness,
            RunMeasurements::GatewayLiveness { .. } => TestKind::GatewayLiveness,
            RunMeasurements::MixnodeStress { .. } => TestKind::MixnodeStress,
        }
    }

    /// Every measurement the run carries, for a consumer that treats them all alike.
    pub fn all(&self) -> Vec<&InterfaceMeasurement> {
        match self {
            RunMeasurements::MixnodeLiveness { mix_forwarding }
            | RunMeasurements::MixnodeStress { mix_forwarding } => vec![mix_forwarding],
            RunMeasurements::GatewayLiveness {
                client_ingest,
                client_delivery,
            } => vec![client_ingest, client_delivery],
        }
    }
}

/// The counts and timings gathered against ONE of a node's interfaces.
///
/// Fields are populated incrementally as the test progresses; absent values (`None`) indicate
/// that the corresponding step was not reached or did not produce a result.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceMeasurement {
    /// Duration of the Noise handshake on the ingress (responder) side, if completed.
    #[serde(default, with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>))]
    pub ingress_noise_handshake: Option<Duration>,

    /// Duration of the Noise handshake on the egress (initiator) side, if completed.
    #[serde(default, with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>))]
    pub egress_noise_handshake: Option<Duration>,

    /// The (constant) delay of the sphinx packet set during the test run.
    #[serde(default, with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub sphinx_packet_delay: Duration,

    /// Number of sphinx packets successfully sent to the node under test.
    pub packets_sent: usize,

    /// Number of sphinx packets returned by the node and successfully received.
    pub packets_received: usize,

    /// Round-trip time of the very first probe packet, sent in isolation before any load is applied.
    /// Because the node is idle at this point, this value approximates the baseline network latency
    /// to the node without any queuing or processing overhead from the stress test itself.
    /// `None` if the initial probe did not complete successfully.
    #[serde(default, with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>))]
    pub approximate_latency: Option<Duration>,

    /// RTT statistics computed over all received packets, or `None` if no packets were received.
    pub packets_statistics: Option<LatencyDistribution>,

    /// Whether any packet was received with an ID that had already been seen in this test run.
    /// Duplicates should never occur under normal operation; their presence may indicate a
    /// misbehaving or malicious node replaying packets.
    pub received_duplicates: bool,
}

impl InterfaceMeasurement {
    /// Delivery ratio for this interface, clamped to `[0.0, 1.0]`. A measurement that sent nothing
    /// scores zero rather than being treated as absent: a node that could not be measured must not
    /// score better than one measured as broken.
    pub fn received_ratio(&self) -> f64 {
        if self.packets_sent == 0 {
            return 0.0;
        }
        let received = self.packets_received.min(self.packets_sent);
        received as f64 / self.packets_sent as f64
    }
}

/// Captures the outcome of a single test run against a nym node: the run-level facts plus the
/// measurements its kind defines.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    /// Total duration of the test run, including the time it took to establish the connections.
    /// Covers every measurement, since a gateway run holds one session open across both phases.
    #[serde(default, with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>))]
    pub time_taken: Duration,

    /// Human-readable description of the first error that caused the test to abort if any.
    /// Run-level rather than per-measurement: an aborted run stops the whole test.
    pub error: Option<String>,

    /// What the run measured, which also says which kind it was. A phase that produced nothing is
    /// still reported, as a zeroed measurement, so the denominator downstream stays fixed.
    pub measurements: RunMeasurements,
}

impl TestRunResult {
    /// The kind this run was performed as.
    pub fn kind(&self) -> TestKind {
        self.measurements.kind()
    }
}

/// Confirmation returned to an agent after a successful result submission.
/// Currently empty — exists to give the response an explicit type rather than
/// relying on `Json(())`.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunSubmissionResponse {}

// ------------------------------------------------------------------------
// Response shapes for the read-only results API (`/v1/results/*`). These are
// the public, serialisation-stable types returned to callers; conversion from
// the storage layer's sqlx rows happens in `orchestrator/storage/models.rs`.
// ------------------------------------------------------------------------

pub const PAGINATION_SIZE_DEFAULT: usize = 50;
pub const PAGINATION_SIZE_MAX: usize = 200;
pub const PAGINATION_PAGE_DEFAULT: usize = 0;

/// Query parameters for paginated endpoints. `size` defaults to
/// [`PAGINATION_SIZE_DEFAULT`] and is capped at [`PAGINATION_SIZE_MAX`];
/// `page` defaults to [`PAGINATION_PAGE_DEFAULT`].
#[cfg_attr(feature = "openapi", derive(utoipa::IntoParams))]
#[cfg_attr(feature = "openapi", into_params(parameter_in = Query))]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub per_page: Option<usize>,
    pub page: Option<usize>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            per_page: Some(PAGINATION_SIZE_DEFAULT),
            page: Some(PAGINATION_PAGE_DEFAULT),
        }
    }
}

impl Pagination {
    pub fn new(per_page: Option<usize>, page: Option<usize>) -> Self {
        Self { per_page, page }
    }

    /// Resolved page size — defaults to [`PAGINATION_SIZE_DEFAULT`] when absent
    /// and is capped at [`PAGINATION_SIZE_MAX`].
    pub fn per_page(&self) -> usize {
        self.per_page
            .unwrap_or(PAGINATION_SIZE_DEFAULT)
            .min(PAGINATION_SIZE_MAX)
    }

    /// Resolved page index — defaults to [`PAGINATION_PAGE_DEFAULT`] when absent.
    pub fn page(&self) -> usize {
        self.page.unwrap_or(PAGINATION_PAGE_DEFAULT)
    }

    /// Value to bind to a SQL `LIMIT ?` clause. Equivalent to
    /// [`Self::per_page`] cast to the `i64` sqlx bind type.
    pub fn limit(&self) -> i64 {
        self.per_page() as i64
    }

    /// Value to bind to a SQL `OFFSET ?` clause, i.e. `page * per_page`.
    /// Saturating to avoid overflow on absurdly large `page` values from a client.
    pub fn offset(&self) -> i64 {
        (self.page() as i64).saturating_mul(self.limit())
    }
}

/// Generic wrapper for a single page of results.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PagedResult<T> {
    pub page: usize,
    pub per_page: usize,
    pub total: usize,
    pub items: Vec<T>,
}

/// A completed test run as exposed by the results API.
///
/// Unlike the agent-facing [`TestRunResult`], this carries the database id,
/// the node that was tested, and the timestamp at which the result was
/// recorded by the orchestrator.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunData {
    /// Database-assigned identifier of the test run.
    pub id: i64,

    /// Node that was tested.
    pub node_id: u32,

    /// The address of that node that was tested.
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub tested_address: SocketAddr,

    /// When the test run completed and was recorded.
    /// Serialised as an RFC 3339 timestamp string.
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub test_timestamp: OffsetDateTime,

    /// The test run result itself.
    pub result: TestRunResult,
}

/// Public snapshot of a nym-node as tracked by the orchestrator: its on-chain bond, plus what the
/// node last reported about itself.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NymNodeData {
    pub node_id: u32,

    /// Ed25519 identity key of the node, serialised as a base58 string.
    #[serde(with = "bs58_ed25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub identity_key: ed25519::PublicKey,

    /// When this node was last observed as bonded in the contract.
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub last_seen_bonded: OffsetDateTime,

    /// `None` until the node has answered a refresh completely, and again once it is no longer
    /// bonded. A node without one is never tested.
    pub description: Option<NymNodeDescriptionData>,
}

/// What a node last reported about itself through its own endpoint.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NymNodeDescriptionData {
    /// Port of the node's mixnet listener, on each of its announced addresses.
    pub mix_port: u16,

    /// Every ip address the node announced, each of which is tested in turn.
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub announced_ips: Vec<IpAddr>,

    /// X25519 public key used for Noise handshakes.
    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub noise_key: x25519::PublicKey,

    /// Sphinx public key used for packet encryption, and the key rotation epoch it belongs to.
    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub sphinx_key: x25519::PublicKey,
    pub key_rotation_id: u32,

    /// The roles the node reports. `gateway_enabled` is the entry-gateway role.
    pub mixnode_enabled: bool,
    pub gateway_enabled: bool,

    /// Port of the node's plain client websocket listener. Present exactly when
    /// `gateway_enabled` is set.
    pub clients_ws_port: Option<u16>,
}

/// Node snapshot paired with its most recent completed run of each kind.
///
/// A field is `None` when the node has never been tested by that kind, or when its most recent
/// run has been evicted by the stale-result sweeper.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NymNodeWithTestRuns {
    pub node: NymNodeData,

    pub latest_mixnode_liveness: Option<TestRunData>,
    pub latest_gateway_liveness: Option<TestRunData>,
    pub latest_mixnode_stress: Option<TestRunData>,
}

/// Marker for a test run that has been handed out to an agent but whose result
/// hasn't been submitted yet. Stripped of test-payload fields because by
/// definition none of them exist yet - what it does carry is what the
/// orchestrator chose when it dispatched the run.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunInProgressData {
    pub node_id: u32,

    /// What this run was dispatched to measure, which for a dual-role node
    /// also says which of its roles.
    pub test_kind: TestKind,

    /// When the test run was handed out to an agent. Serialised as an
    /// RFC 3339 timestamp string.
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub started_at: OffsetDateTime,

    /// When the lease expires and the node is freed for reassignment whether or
    /// not a result ever arrives. A row already past this is waiting on the next
    /// eviction sweep rather than still being worked on, which is what tells a
    /// slow run apart from an abandoned one.
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub expires_at: OffsetDateTime,
}

/// The evidence behind one kind's aggregate: the value and how many runs it was averaged over.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KindAggregate {
    /// Mean of the per-run scores in the window, in `[0.0, 1.0]`.
    pub score: f64,

    /// How many runs that mean was taken over, which is the only ground truth about how much
    /// evidence stands behind the score.
    pub samples: u32,
}

/// One node's materialised aggregates for one mixnet epoch, a separate liveness and stress entry.
///
/// An entry with no value in the window is `None` rather than a zero, so an unmeasured node stays
/// distinguishable from one measured at zero. Entries are named fields rather than a map because a
/// further sibling is expected - config score is the next to move out of nym-api - and it will
/// carry its own shape rather than this score-and-count pair, so the entries cannot share one value
/// type. A new sibling is a new optional field, which is additive.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NodeEpochAggregates {
    pub node_id: u32,

    pub mixnet_epoch: u32,

    /// The liveness aggregate over the node's mixnode and gateway liveness runs together, or `None`
    /// if neither kind has a run for this node in the window.
    pub liveness: Option<KindAggregate>,

    /// The stress aggregate, or `None` if no stress run exists for this node in the window.
    pub stress: Option<KindAggregate>,
}

impl NodeEpochAggregates {
    /// Assembles a node's record from each kind already extracted. A further sibling is a further
    /// argument, free to be its own type and sourced from its own place, rather than another arm in
    /// a loop over one row list.
    pub fn new(
        node_id: u32,
        mixnet_epoch: u32,
        liveness: Option<KindAggregate>,
        stress: Option<KindAggregate>,
    ) -> Self {
        NodeEpochAggregates {
            node_id,
            mixnet_epoch,
            liveness,
            stress,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addresses(v4: &str, v6: &str) -> AgentMixAddresses {
        AgentMixAddresses {
            v4: v4.parse().unwrap(),
            v6: v6.parse().unwrap(),
        }
    }

    #[test]
    fn a_plain_address_of_each_family_has_distinct_families() {
        assert!(addresses("1.1.1.1:1789", "[aaaa::1]:1789").has_distinct_families());
    }

    #[test]
    fn swapped_or_duplicated_families_do_not() {
        assert!(!addresses("[aaaa::1]:1789", "1.1.1.1:1789").has_distinct_families());
        assert!(!addresses("1.1.1.1:1789", "1.1.1.1:1789").has_distinct_families());
        assert!(!addresses("[aaaa::1]:1789", "[aaaa::1]:1789").has_distinct_families());
    }

    // nodes store the authorised agent addresses under their canonical form, so an ipv4-mapped
    // address in the v6 field collapses onto the v4 one instead of authorising a second ingress
    fn mixnet_target() -> MixnetProbeTarget {
        let mut rng = nym_test_utils::helpers::deterministic_rng();
        let x_key = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut rng));
        MixnetProbeTarget {
            node_id: 42,
            identity_key: *ed25519::KeyPair::new(&mut rng).public_key(),
            node_address: "1.1.1.1:1789".parse().unwrap(),
            node_ips: vec!["1.1.1.1".parse().unwrap(), "aaaa::1".parse().unwrap()],
            noise_key: x_key,
            sphinx_key: x_key,
            key_rotation_id: 7,
        }
    }

    #[test]
    fn a_stress_assignment_round_trips_as_a_single_target() {
        let json =
            serde_json::to_string(&TestRunAssignment::MixnodeStress(Box::new(mixnet_target())))
                .unwrap();
        assert!(json.contains(r#"{"mixnode_stress":{"#), "{json}");

        let parsed: TestRunAssignment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind(), TestKind::MixnodeStress);

        let TestRunAssignment::MixnodeStress(target) = parsed else {
            panic!("round-tripped into the wrong variant: {json}");
        };
        assert_eq!(target.node_id, 42);
        assert_eq!(target.key_rotation_id, 7);
    }

    // The assignment is EXTERNALLY tagged, which is load-bearing rather than stylistic: a liveness
    // variant carries a WAVE, and serde cannot internally tag a sequence. Switching to
    // `#[serde(tag = ...)]` would compile and then fail at runtime for exactly these variants, so
    // pin that a wave serialises as an array under its tag.
    #[test]
    fn a_liveness_assignment_round_trips_as_a_wave() {
        let wave = vec![mixnet_target(), mixnet_target()];
        let json = serde_json::to_string(&TestRunAssignment::MixnodeLiveness(wave)).unwrap();
        assert!(json.contains(r#"{"mixnode_liveness":[{"#), "{json}");

        let parsed: TestRunAssignment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind(), TestKind::MixnodeLiveness);

        let TestRunAssignment::MixnodeLiveness(wave) = parsed else {
            panic!("round-tripped into the wrong variant: {json}");
        };
        assert_eq!(wave.len(), 2);
    }

    #[test]
    fn a_gateway_wave_keeps_its_nested_mixnet_target_and_ws_port() {
        let wave = vec![GatewayProbeTarget {
            mixnet: mixnet_target(),
            clients_ws_port: 9000,
        }];
        let json = serde_json::to_string(&TestRunAssignment::GatewayLiveness(wave)).unwrap();
        assert!(json.contains(r#"{"gateway_liveness":[{"#), "{json}");

        let parsed: TestRunAssignment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind(), TestKind::GatewayLiveness);

        let TestRunAssignment::GatewayLiveness(wave) = parsed else {
            panic!("round-tripped into the wrong variant: {json}");
        };
        assert_eq!(wave[0].clients_ws_port, 9000);
        // the egress phase targets the node's mixnet listener, so the nested target has to survive
        assert_eq!(wave[0].mixnet.node_id, 42);
        assert_eq!(wave[0].mixnet.key_rotation_id, 7);
    }

    // The two mixnode probes carry the SAME per-target payload and differ only in tag and arity,
    // so nothing in the target itself can tell the agent which profile to apply.
    #[test]
    fn the_tag_is_what_distinguishes_the_two_mixnode_probes() {
        let target_json = serde_json::to_string(&mixnet_target()).unwrap();

        let stress =
            serde_json::to_string(&TestRunAssignment::MixnodeStress(Box::new(mixnet_target())))
                .unwrap();
        let liveness =
            serde_json::to_string(&TestRunAssignment::MixnodeLiveness(vec![mixnet_target()]))
                .unwrap();

        assert!(stress.contains(&target_json), "{stress}");
        assert!(liveness.contains(&target_json), "{liveness}");
        assert_ne!(stress, liveness);
    }

    // deliberately awkward values: each field gets a distinct one so a transposition is caught, and
    // the durations carry nanosecond remainders because every one of them crosses the wire through
    // `humantime_serde`, where silent precision loss would corrupt the measurement rather than fail
    fn distribution(seed: u64) -> LatencyDistribution {
        LatencyDistribution {
            minimum: Duration::from_nanos(seed * 1_000 + 1),
            mean: Duration::from_nanos(seed * 2_000 + 2),
            median: Duration::from_nanos(seed * 3_000 + 3),
            maximum: Duration::from_nanos(seed * 4_000 + 4),
            standard_deviation: Duration::from_nanos(seed * 5_000 + 5),
        }
    }

    fn measurement(sent: usize, received: usize) -> InterfaceMeasurement {
        InterfaceMeasurement {
            ingress_noise_handshake: Some(Duration::from_micros(1_234)),
            egress_noise_handshake: Some(Duration::from_micros(5_678)),
            sphinx_packet_delay: Duration::from_millis(50),
            packets_sent: sent,
            packets_received: received,
            approximate_latency: Some(Duration::from_nanos(1_500_250)),
            packets_statistics: Some(distribution(1)),
            received_duplicates: false,
        }
    }

    #[test]
    fn a_gateway_liveness_run_round_trips_both_of_its_measurements() {
        let run = TestRunResult {
            time_taken: Duration::from_millis(2_500),
            error: None,
            measurements: RunMeasurements::GatewayLiveness {
                client_ingest: measurement(100, 100),
                client_delivery: measurement(100, 0),
            },
        };

        let json = serde_json::to_string(&run).unwrap();
        let parsed: TestRunResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.kind(), TestKind::GatewayLiveness);

        // the healthy phase must not be readable as the dead one. this is the whole reason the two
        // are kept apart instead of averaged at the agent
        let RunMeasurements::GatewayLiveness {
            client_ingest,
            client_delivery,
        } = &parsed.measurements
        else {
            panic!("round-tripped into the wrong kind: {json}");
        };
        assert_eq!(client_ingest.received_ratio(), 1.0);
        assert_eq!(client_delivery.received_ratio(), 0.0);

        // re-serialising reproduces the bytes, so nothing was dropped, reordered or rounded
        assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
    }

    #[test]
    fn a_single_measurement_run_round_trips_unchanged() {
        let run = TestRunResult {
            time_taken: Duration::from_secs(30),
            error: Some("connection reset".to_string()),
            measurements: RunMeasurements::MixnodeStress {
                mix_forwarding: measurement(10_000, 9_997),
            },
        };

        let json = serde_json::to_string(&run).unwrap();
        let parsed: TestRunResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.kind(), TestKind::MixnodeStress);
        assert_eq!(parsed.time_taken, Duration::from_secs(30));
        assert_eq!(parsed.error.as_deref(), Some("connection reset"));

        let RunMeasurements::MixnodeStress {
            mix_forwarding: measured,
        } = &parsed.measurements
        else {
            panic!("round-tripped into the wrong kind: {json}");
        };
        assert_eq!(measured.packets_sent, 10_000);
        assert_eq!(measured.packets_received, 9_997);
        assert_eq!(
            measured.ingress_noise_handshake,
            Some(Duration::from_micros(1_234))
        );
        assert_eq!(
            measured.egress_noise_handshake,
            Some(Duration::from_micros(5_678))
        );
        assert_eq!(measured.sphinx_packet_delay, Duration::from_millis(50));
        // sub-millisecond value with a nanosecond remainder, intact
        assert_eq!(
            measured.approximate_latency,
            Some(Duration::from_nanos(1_500_250))
        );
        assert_eq!(
            measured.packets_statistics.unwrap().median,
            Duration::from_nanos(3_003)
        );
        assert_eq!(
            measured.packets_statistics.unwrap().standard_deviation,
            Duration::from_nanos(5_005)
        );

        assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
    }

    #[test]
    fn an_ipv4_mapped_v6_address_does_not() {
        assert!(!addresses("1.1.1.1:1789", "[::ffff:1.1.1.1]:1789").has_distinct_families());
        assert!(!addresses("1.1.1.1:1789", "[::ffff:2.2.2.2]:1789").has_distinct_families());
    }
}
