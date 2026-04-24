// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::pagination::PaginatedResponse;
use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_mixnet_contract_common::NodeId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, Default, ToSchema)]
pub struct TestNode {
    pub node_id: Option<u32>,
    pub identity_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct TestRoute {
    pub gateway: TestNode,
    pub layer1: TestNode,
    pub layer2: TestNode,
    pub layer3: TestNode,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct PartialTestResult {
    pub monitor_run_id: i64,
    pub timestamp: i64,
    pub overall_reliability_for_all_routes_in_monitor_run: Option<u8>,
    pub test_routes: TestRoute,
}

pub type MixnodeTestResultResponse = PaginatedResponse<PartialTestResult>;
pub type GatewayTestResultResponse = PaginatedResponse<PartialTestResult>;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NetworkMonitorRunDetailsResponse {
    pub monitor_run_id: i64,
    pub network_reliability: f64,
    pub total_sent: usize,
    pub total_received: usize,

    // integer score to number of nodes with that score
    pub mixnode_results: BTreeMap<u8, usize>,
    pub gateway_results: BTreeMap<u8, usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/MixnodeCoreStatusResponse.ts"
    )
)]
pub struct MixnodeCoreStatusResponse {
    pub mix_id: NodeId,
    pub count: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/GatewayCoreStatusResponse.ts"
    )
)]
pub struct GatewayCoreStatusResponse {
    pub identity: String,
    pub count: i64,
}

pub use v3::*;

/// Request/response types for the v3 network-monitor flow, in which an orchestrator submits
/// stress testing results to nym-api via signed batches.
pub mod v3 {
    use super::*;
    use crate::signable::SignedMessage;
    use std::time::Duration;
    use time::OffsetDateTime;

    /// Signed envelope posted by a network monitor orchestrator to
    /// `POST /v3/nym-nodes/stress-testing/batch-submit`.
    ///
    /// The signature is checked against the `signer` field of the inner
    /// [`StressTestBatchSubmissionContent`], which must also match one of the orchestrators
    /// registered in the network-monitors contract.
    pub type StressTestBatchSubmission = SignedMessage<StressTestBatchSubmissionContent>;

    /// Single stress-test measurement for one node, produced by a network monitor orchestrator.
    #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
    pub struct StressTestResult {
        /// Contract-assigned id of the node that was tested.
        pub node_id: NodeId,

        /// Whether the tested node was acting as a mixnode during the measurement.
        ///
        /// Included explicitly (rather than inferred from on-chain role) so the API can reject or
        /// route entries that don't match the expected role without re-querying the contract.
        pub is_mixnode: bool,

        #[schema(value_type = String)]
        #[serde(with = "time::serde::rfc3339")]
        pub test_timestamp: OffsetDateTime,

        /// Measured performance score in the `[0.0, 1.0]` range.
        pub test_performance: f64,

        /// Whether the node responded at all during testing.
        ///
        /// Recorded alongside `test_performance` so that a genuine 0.0 score (node responded but
        /// dropped everything) can be distinguished from the node being offline entirely.
        pub was_reachable: bool,
    }

    /// Body of a stress-test batch submission, signed by a network monitor orchestrator.
    #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
    pub struct StressTestBatchSubmissionContent {
        /// ed25519 identity key of the submitting orchestrator. Must match an entry in the
        /// network-monitors contract for the batch to be accepted.
        #[schema(value_type = String)]
        #[serde(with = "ed25519::bs58_ed25519_pubkey")]
        pub signer: ed25519::PublicKey,

        /// Time at which this batch was produced. Also used as a monotonic nonce for replay
        /// protection: the API rejects submissions whose timestamp is not strictly greater than
        /// the orchestrator's previous accepted submission.
        #[schema(value_type = String)]
        #[serde(with = "time::serde::rfc3339")]
        pub timestamp: OffsetDateTime,

        pub results: Vec<StressTestResult>,
    }

    impl StressTestBatchSubmissionContent {
        /// Build a batch submission body stamped with the current UTC time.
        pub fn new(signer: ed25519::PublicKey, results: Vec<StressTestResult>) -> Self {
            StressTestBatchSubmissionContent {
                signer,
                timestamp: OffsetDateTime::now_utc(),
                results,
            }
        }

        /// Whether this submission is older than `max_age` relative to the current UTC time.
        ///
        /// Used server-side to reject submissions that have been sitting around too long, even if
        /// they are otherwise well-formed and correctly signed.
        pub fn is_stale(&self, max_age: Duration) -> bool {
            self.timestamp + max_age < OffsetDateTime::now_utc()
        }
    }

    /// Response body for `GET /v3/nym-nodes/stress-testing/known-monitors/{identity_key}`,
    /// used by orchestrators to check whether this nym-api currently recognises their key.
    #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
    pub struct KnownNetworkMonitorResponse {
        /// The ed25519 identity key that was queried (base58-encoded on the wire).
        #[serde(with = "bs58_ed25519_pubkey")]
        #[schema(value_type = String)]
        pub identity_key: ed25519::PublicKey,

        /// Whether the queried identity key is currently recognised by this nym-api
        /// as an authorised network monitor permitted to submit stress testing results.
        pub authorised: bool,
    }
}
