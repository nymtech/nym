// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_crypto::asymmetric::x25519;
use nym_crypto::asymmetric::x25519::serde_helpers::{
    bs58_x25519_pubkey, option_bs58_x25519_pubkey,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
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

/// What a test run measures. Orthogonal to [`TestedRole`], which is the role the node was probed
/// in: a `liveness` run of a dual-role node is one run per role.
///
/// Deliberately has no `Default` - the kind decides eligibility, cadence and the expected signal
/// set, so a silently defaulted value would measure the wrong thing rather than fail.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestKind {
    /// High-volume throughput probe, one target per assignment.
    Stress,

    /// Low-volume delivery-ratio probe, a wave of targets per assignment.
    Liveness,
}

impl TestKind {
    /// The kind's canonical string form, shared by its JSON tag, its stored column value and its
    /// prometheus label so the three cannot drift apart.
    pub fn as_str(&self) -> &'static str {
        match self {
            TestKind::Stress => "stress",
            TestKind::Liveness => "liveness",
        }
    }
}

impl fmt::Display for TestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Response from the orchestrator when an agent requests work.
/// `assignment` is `None` when no nodes are due for testing.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunAssignmentResponse {
    pub assignment: Option<TestRunAssignment>,
}

/// Work handed to an agent, tagged by what is being measured and in which role.
///
/// The variants correspond to the ([`TestKind`], [`TestedRole`]) pairs the orchestrator may
/// assign, and are named for both because the pairing is not one-to-one: a gateway stress test
/// would not resemble the mixnode one. `MixnodeStress` and `MixnodeLiveness` carry the same
/// payload because they are the same probe, differing only in the profile the agent applies, which
/// the agent holds in its own config and selects from the tag. `GatewayLiveness` additionally
/// carries what is needed to open a client websocket session.
///
/// A stress assignment is ONE target; a liveness assignment is a WAVE the agent probes
/// concurrently, so the lease the orchestrator stamps is bounded by the slowest single target
/// rather than by their sum. A wave is homogeneous in role, because the two liveness probes are
/// different machinery: a dual-role node is assigned each role separately.
///
/// An assignment with no targets is NOT a valid assignment. "No work" is expressed by an absent
/// assignment on [`TestRunAssignmentResponse`], so the orchestrator must not emit an empty wave.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRunAssignment {
    MixnodeStress(MixnetProbeTarget),
    MixnodeLiveness(Vec<MixnetProbeTarget>),
    GatewayLiveness(Vec<GatewayProbeTarget>),
}

impl TestRunAssignment {
    /// What this assignment measures. Determines the profile the agent applies, and the kind
    /// recorded against the resulting run.
    pub fn kind(&self) -> TestKind {
        match self {
            TestRunAssignment::MixnodeStress(_) => TestKind::Stress,
            TestRunAssignment::MixnodeLiveness(_) | TestRunAssignment::GatewayLiveness(_) => {
                TestKind::Liveness
            }
        }
    }

    /// The role every node in this assignment is probed in. A dual-role node is assigned each role
    /// separately, so this is a property of the assignment rather than of the node.
    pub fn tested_role(&self) -> TestedRole {
        match self {
            TestRunAssignment::MixnodeStress(_) | TestRunAssignment::MixnodeLiveness(_) => {
                TestedRole::Mixnode
            }
            TestRunAssignment::GatewayLiveness(_) => TestedRole::Gateway,
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

/// Latency statistics computed over the set of test packets received or sent during a stress test.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatencyDistribution {
    /// Minimum latency duration it took to send or receive a test packet.
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub minimum: Duration,

    /// Average latency duration it took to send or receive a test packet.
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub mean: Duration,

    /// Median latency duration it took to send or receive a test packet.
    /// For an even number of samples, this is the arithmetic mean of the two middle values.
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub median: Duration,

    /// Maximum latency duration it took to send or receive a test packet.
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub maximum: Duration,

    /// The standard deviation of the latency duration it took to send or receive the test packets.
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

/// Which of the node's packet-handling interfaces a set of counts exercised.
///
/// Names the node FUNCTION under measurement rather than a route, because every value traverses
/// the mixnet in some form and so a route-shaped name would not distinguish them. The mixnode
/// probe exercises one interface and so produces only [`ExercisedInterface::MixForwarding`]; the
/// gateway probe exercises two, kept separate because averaging them at the agent would make a
/// healthy ingest with a dead delivery indistinguishable from a uniformly half-lossy node.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExercisedInterface {
    /// The node forwarding as a mixing hop, measured by the two-hop self-loop through its mixnet
    /// listener.
    MixForwarding,

    /// The node accepting packets from a client session and injecting them into the mixnet.
    ClientIngest,

    /// The node taking final-hop packets off the mixnet and delivering them to a client session.
    ClientDelivery,
}

impl ExercisedInterface {
    /// The interface's canonical string form, shared by its JSON tag, its stored column value and
    /// its prometheus label so the three cannot drift apart.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExercisedInterface::MixForwarding => "mix_forwarding",
            ExercisedInterface::ClientIngest => "client_ingest",
            ExercisedInterface::ClientDelivery => "client_delivery",
        }
    }
}

impl fmt::Display for ExercisedInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The counts and timings gathered against ONE of a node's interfaces.
///
/// Fields are populated incrementally as the test progresses; absent values (`None`) indicate
/// that the corresponding step was not reached or did not produce a result.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceMeasurement {
    /// Which interface these counts describe.
    pub interface: ExercisedInterface,

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

    /// Latency distribution of individual batch send operations recorded during the load test.
    /// Reflects how long each batch took to flush to the OS socket, giving a rough measure of
    /// egress throughput. `None` if no batches were sent.
    pub sending_statistics: Option<LatencyDistribution>,

    /// Whether any packet was received with an ID that had already been seen in this test run.
    /// Duplicates should never occur under normal operation; their presence may indicate a
    /// misbehaving or malicious node replaying packets.
    pub received_duplicates: bool,
}

impl InterfaceMeasurement {
    /// A measurement with nothing recorded yet, which is also what a phase that never ran reports:
    /// zero sent, zero received, hence a zero delivery ratio.
    pub fn new(interface: ExercisedInterface, sphinx_packet_delay: Duration) -> Self {
        InterfaceMeasurement {
            interface,
            ingress_noise_handshake: None,
            egress_noise_handshake: None,
            sphinx_packet_delay,
            packets_sent: 0,
            packets_received: 0,
            approximate_latency: None,
            packets_statistics: None,
            sending_statistics: None,
            received_duplicates: false,
        }
    }

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

/// Captures the outcome of a single test run against a nym node: the run-level facts plus one
/// measurement per interface the run exercised.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    /// What this run measured. Echoed back from the assignment so the orchestrator records the run
    /// under the kind it handed out.
    pub kind: TestKind,

    /// Total duration of the test run, including the time it took to establish the connections.
    /// Covers every measurement, since a gateway run holds one session open across both phases.
    #[serde(default, with = "humantime_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>))]
    pub time_taken: Duration,

    /// Human-readable description of the first error that caused the test to abort if any.
    /// Run-level rather than per-measurement: an aborted run stops the whole test.
    pub error: Option<String>,

    /// One entry per interface exercised. A mixnode probe produces exactly one; the gateway probe
    /// produces one per phase. A phase that produced nothing is still reported, as a zeroed
    /// measurement, so the denominator downstream stays fixed.
    pub measurements: Vec<InterfaceMeasurement>,
}

impl TestRunResult {
    /// The measurement for a given interface, if this run exercised it.
    pub fn measurement(&self, interface: ExercisedInterface) -> Option<&InterfaceMeasurement> {
        self.measurements.iter().find(|m| m.interface == interface)
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

/// The role a node was probed in by a test run. Distinct from the node's own capability
/// classification, which may be both.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestedRole {
    Mixnode,
    Gateway,
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

    /// The address of that node that was tested. `None` for runs recorded before the orchestrator
    /// started tracking it.
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>))]
    pub tested_address: Option<SocketAddr>,

    /// The role the node was probed in.
    pub tested_role: TestedRole,

    /// When the test run completed and was recorded.
    /// Serialised as an RFC 3339 timestamp string.
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub test_timestamp: OffsetDateTime,

    /// The test run result itself.
    pub result: TestRunResult,
}

/// Public snapshot of a nym-node as tracked by the orchestrator.
///
/// Built from the on-chain bond plus any details the orchestrator has managed
/// to retrieve directly from the node itself. The optional fields
/// (`mixnet_socket_address`, `noise_key`, `sphinx_key`, `key_rotation_id`)
/// are populated lazily by the node refresher and may be absent either because
/// the node is newly observed or because the refresher failed to reach it.
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

    /// Mixnet socket address (host:port) at which the node accepts sphinx packets.
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub mixnet_socket_address: Option<SocketAddr>,

    /// X25519 public key used for Noise handshakes.
    /// `None` if retrieval from the node failed.
    #[serde(with = "option_bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub noise_key: Option<x25519::PublicKey>,

    /// Sphinx public key used for packet encryption.
    /// `None` if retrieval from the node failed.
    /// Always `None`/`Some` together with `key_rotation_id`.
    #[serde(with = "option_bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub sphinx_key: Option<x25519::PublicKey>,

    /// Key rotation epoch ID that `sphinx_key` belongs to.
    /// `None` if retrieval from the node failed.
    /// Always `None`/`Some` together with `sphinx_key`.
    pub key_rotation_id: Option<i64>,
}

/// Node snapshot paired with its most recent completed test run.
///
/// `latest_test_run` is `None` when the node has never been tested or when its
/// most recent run has been evicted by the stale-result sweeper.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NymNodeWithTestRun {
    pub node: NymNodeData,

    pub latest_test_run: Option<TestRunData>,
}

/// Marker for a test run that has been handed out to an agent but whose result
/// hasn't been submitted yet. Stripped of test-payload fields because by
/// definition none of them exist yet.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunInProgressData {
    pub node_id: u32,

    /// When the test run was handed out to an agent. Serialised as an
    /// RFC 3339 timestamp string.
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub started_at: OffsetDateTime,
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
            serde_json::to_string(&TestRunAssignment::MixnodeStress(mixnet_target())).unwrap();
        assert!(json.contains(r#"{"mixnode_stress":{"#), "{json}");

        let parsed: TestRunAssignment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind(), TestKind::Stress);
        assert_eq!(parsed.tested_role(), TestedRole::Mixnode);

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
        assert_eq!(parsed.kind(), TestKind::Liveness);
        assert_eq!(parsed.tested_role(), TestedRole::Mixnode);

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
        assert_eq!(parsed.kind(), TestKind::Liveness);
        assert_eq!(parsed.tested_role(), TestedRole::Gateway);

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
            serde_json::to_string(&TestRunAssignment::MixnodeStress(mixnet_target())).unwrap();
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

    fn measurement(
        interface: ExercisedInterface,
        sent: usize,
        received: usize,
    ) -> InterfaceMeasurement {
        InterfaceMeasurement {
            interface,
            ingress_noise_handshake: Some(Duration::from_micros(1_234)),
            egress_noise_handshake: Some(Duration::from_micros(5_678)),
            sphinx_packet_delay: Duration::from_millis(50),
            packets_sent: sent,
            packets_received: received,
            approximate_latency: Some(Duration::from_nanos(1_500_250)),
            packets_statistics: Some(distribution(1)),
            sending_statistics: Some(distribution(2)),
            received_duplicates: false,
        }
    }

    #[test]
    fn a_gateway_liveness_run_round_trips_both_of_its_measurements() {
        let run = TestRunResult {
            kind: TestKind::Liveness,
            time_taken: Duration::from_millis(2_500),
            error: None,
            measurements: vec![
                measurement(ExercisedInterface::ClientIngest, 100, 100),
                measurement(ExercisedInterface::ClientDelivery, 100, 0),
            ],
        };

        let json = serde_json::to_string(&run).unwrap();
        let parsed: TestRunResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.kind, TestKind::Liveness);
        assert_eq!(parsed.measurements.len(), 2);

        // order is preserved, so the healthy phase cannot be read as the dead one. this is the
        // whole reason the two are kept apart instead of averaged at the agent
        assert_eq!(
            parsed.measurements[0].interface,
            ExercisedInterface::ClientIngest
        );
        assert_eq!(
            parsed.measurements[1].interface,
            ExercisedInterface::ClientDelivery
        );
        assert_eq!(parsed.measurements[0].received_ratio(), 1.0);
        assert_eq!(parsed.measurements[1].received_ratio(), 0.0);

        // and each is reachable by interface rather than by position
        assert_eq!(
            parsed
                .measurement(ExercisedInterface::ClientDelivery)
                .unwrap()
                .packets_received,
            0
        );
        assert!(
            parsed
                .measurement(ExercisedInterface::MixForwarding)
                .is_none()
        );

        // re-serialising reproduces the bytes, so nothing was dropped, reordered or rounded
        assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
    }

    #[test]
    fn a_single_measurement_run_round_trips_unchanged() {
        let run = TestRunResult {
            kind: TestKind::Stress,
            time_taken: Duration::from_secs(30),
            error: Some("connection reset".to_string()),
            measurements: vec![measurement(
                ExercisedInterface::MixForwarding,
                10_000,
                9_997,
            )],
        };

        let json = serde_json::to_string(&run).unwrap();
        let parsed: TestRunResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.kind, TestKind::Stress);
        assert_eq!(parsed.time_taken, Duration::from_secs(30));
        assert_eq!(parsed.error.as_deref(), Some("connection reset"));
        assert_eq!(parsed.measurements.len(), 1);

        let measured = &parsed.measurements[0];
        assert_eq!(measured.interface, ExercisedInterface::MixForwarding);
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
            measured.sending_statistics.unwrap().standard_deviation,
            Duration::from_nanos(10_005)
        );

        assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
    }

    // `as_str` feeds the stored column value and the prometheus label while serde produces the
    // wire tag. They must be the same string: if they diverged, rows already stored under one
    // spelling would stop matching queries built from the other, and the drift would be silent.
    #[test]
    fn test_kind_wire_tag_matches_its_string_form() {
        for kind in [TestKind::Stress, TestKind::Liveness] {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
            assert_eq!(kind.to_string(), kind.as_str());

            let parsed: TestKind = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, kind);
        }

        // pin the spellings themselves, so a `rename_all` change fails here rather than in a
        // migration that no longer matches the rows it was written against
        assert_eq!(TestKind::Stress.as_str(), "stress");
        assert_eq!(TestKind::Liveness.as_str(), "liveness");
    }

    #[test]
    fn an_ipv4_mapped_v6_address_does_not() {
        assert!(!addresses("1.1.1.1:1789", "[::ffff:1.1.1.1]:1789").has_distinct_families());
        assert!(!addresses("1.1.1.1:1789", "[::ffff:2.2.2.2]:1789").has_distinct_families());
    }
}
