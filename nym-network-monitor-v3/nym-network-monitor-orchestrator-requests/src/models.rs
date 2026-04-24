// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_crypto::asymmetric::x25519;
use nym_crypto::asymmetric::x25519::serde_helpers::{
    bs58_x25519_pubkey, option_bs58_x25519_pubkey,
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use time::OffsetDateTime;

/// Request sent by an agent to obtain a unique mixnet port from the orchestrator.
/// The orchestrator uses the agent's host IP and noise key to ensure no two agents
/// on the same host are assigned the same port.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPortRequest {
    /// Egress address of the agent node
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub agent_node_ip: IpAddr,

    /// Base-58 encoded noise key of the agent.
    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub x25519_noise_key: x25519::PublicKey,
}

/// Response to an [`AgentPortRequest`], containing the port the agent should bind to.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPortRequestResponse {
    pub available_mix_port: u16,
}

#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Body sent by an agent to announce its details to the orchestrator.
/// The orchestrator forwards this information to the smart contract so that
/// network nodes can whitelist connections from known agents.
pub struct AgentAnnounceRequest {
    /// Egress address of the agent node combined with the previously
    /// assigned mixnet socket address from the orchestrator
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub agent_mix_socket_address: SocketAddr,

    /// Base-58 encoded noise key of the agent.
    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub x25519_noise_key: x25519::PublicKey,

    /// Version of the noise protocol used by the agent.
    pub noise_version: u8,
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
    /// Egress address of the agent node combined with the previously
    /// assigned mixnet socket address from the orchestrator
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub agent_mix_socket_address: SocketAddr,

    /// Base-58 encoded noise key of the agent.
    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub x25519_noise_key: x25519::PublicKey,
}

/// Response from the orchestrator when an agent requests work.
/// `assignment` is `None` when no nodes are due for testing.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunAssignmentResponse {
    pub assignment: Option<TestRunAssignment>,
}

/// Details of a single node assigned to an agent for stress testing.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunAssignment {
    pub node_id: u32,

    /// The address of the node that should be tested.
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub node_address: SocketAddr,

    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub noise_key: x25519::PublicKey,

    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub sphinx_key: x25519::PublicKey,

    pub key_rotation_id: u32,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestRunResultSubmissionRequest {
    pub node_id: u32,
    pub result: TestRunResult,
}

/// Captures the outcome of a single test run against a nym node.
///
/// Fields are populated incrementally as the test progresses; absent values (`None`) indicate
/// that the corresponding step was not reached or did not produce a result.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestRunResult {
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

    /// Human-readable description of the first error that caused the test to abort if any.
    pub error: Option<String>,
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

/// Discriminator for the type of node targeted by a test run.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestType {
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

    /// Kind of node that was tested.
    pub test_type: TestType,

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
