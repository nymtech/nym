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
pub mod v3 {
    use super::*;
    use crate::signable::SignedMessage;
    use std::time::Duration;
    use time::OffsetDateTime;

    pub type StressTestBatchSubmission = SignedMessage<StressTestBatchSubmissionContent>;

    #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
    pub struct StressTestResult {
        pub node_id: NodeId,

        // to explicitly distinguish it from a gateway
        pub is_mixnode: bool,

        #[schema(value_type = String)]
        #[serde(with = "time::serde::rfc3339")]
        pub test_timestamp: OffsetDateTime,

        pub test_performance: f64,

        // distinguish between cases of node having 0 performance and being offline
        pub was_reachable: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
    pub struct StressTestBatchSubmissionContent {
        #[schema(value_type = String)]
        #[serde(with = "ed25519::bs58_ed25519_pubkey")]
        pub signer: ed25519::PublicKey,

        #[schema(value_type = String)]
        #[serde(with = "time::serde::rfc3339")]
        pub timestamp: OffsetDateTime,

        pub results: Vec<StressTestResult>,
    }

    impl StressTestBatchSubmissionContent {
        pub fn new(signer: ed25519::PublicKey, results: Vec<StressTestResult>) -> Self {
            StressTestBatchSubmissionContent {
                signer,
                timestamp: OffsetDateTime::now_utc(),
                results,
            }
        }

        pub fn is_stale(&self, max_age: Duration) -> bool {
            self.timestamp + max_age < OffsetDateTime::now_utc()
        }
    }

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
