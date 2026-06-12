// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::PlaceholderJsonSchemaImpl;
use crate::models::{CoinSchema, DisplayRole};
use crate::pagination::PaginatedResponse;
use cosmwasm_std::{Coin, Decimal};
use nym_contracts_common::{IdentityKey, NaiveFloat};
use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NodeId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;

pub use config_score::*;

pub type StakeSaturation = Decimal;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/StakeSaturationResponse.ts"
    )
)]
pub struct StakeSaturationResponse {
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    #[schema(value_type = String)]
    pub saturation: StakeSaturation,

    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    #[schema(value_type = String)]
    pub uncapped_saturation: StakeSaturation,
    pub as_at: i64,
}

pub mod config_score {
    use nym_contracts_common::NaiveFloat;
    use serde::{Deserialize, Serialize};
    use std::cmp::Ordering;
    use utoipa::ToSchema;

    #[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
    pub struct ConfigScoreDataResponse {
        pub parameters: ConfigScoreParams,
        pub version_history: Vec<HistoricalNymNodeVersionEntry>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
    pub struct HistoricalNymNodeVersionEntry {
        /// The unique, ordered, id of this particular entry
        pub id: u32,

        /// Data associated with this particular version
        pub version_information: HistoricalNymNodeVersion,
    }

    impl PartialOrd for HistoricalNymNodeVersionEntry {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            // we only care about id for the purposes of ordering as they should have unique data
            self.id.partial_cmp(&other.id)
        }
    }

    impl From<nym_mixnet_contract_common::HistoricalNymNodeVersionEntry>
        for HistoricalNymNodeVersionEntry
    {
        fn from(value: nym_mixnet_contract_common::HistoricalNymNodeVersionEntry) -> Self {
            HistoricalNymNodeVersionEntry {
                id: value.id,
                version_information: value.version_information.into(),
            }
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
    pub struct HistoricalNymNodeVersion {
        /// Version of the nym node that is going to be used for determining the version score of a node.
        /// note: value stored here is pre-validated `semver::Version`
        pub semver: String,

        /// Block height of when this version has been added to the contract
        pub introduced_at_height: u64,
        // for now ignore that field. it will give nothing useful to the users
        //     pub difference_since_genesis: TotalVersionDifference,
    }

    impl From<nym_mixnet_contract_common::HistoricalNymNodeVersion> for HistoricalNymNodeVersion {
        fn from(value: nym_mixnet_contract_common::HistoricalNymNodeVersion) -> Self {
            HistoricalNymNodeVersion {
                semver: value.semver,
                introduced_at_height: value.introduced_at_height,
            }
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
    pub struct ConfigScoreParams {
        /// Defines weights for calculating numbers of versions behind the current release.
        pub version_weights: OutdatedVersionWeights,

        /// Defines the parameters of the formula for calculating the version score
        pub version_score_formula_params: VersionScoreFormulaParams,
    }

    /// Defines weights for calculating numbers of versions behind the current release.
    #[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
    pub struct OutdatedVersionWeights {
        pub major: u32,
        pub minor: u32,
        pub patch: u32,
        pub prerelease: u32,
    }

    /// Given the formula of version_score = penalty ^ (versions_behind_factor ^ penalty_scaling)
    /// define the relevant parameters
    #[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
    pub struct VersionScoreFormulaParams {
        pub penalty: f64,
        pub penalty_scaling: f64,
    }

    impl From<nym_mixnet_contract_common::ConfigScoreParams> for ConfigScoreParams {
        fn from(value: nym_mixnet_contract_common::ConfigScoreParams) -> Self {
            ConfigScoreParams {
                version_weights: value.version_weights.into(),
                version_score_formula_params: value.version_score_formula_params.into(),
            }
        }
    }

    impl From<nym_mixnet_contract_common::OutdatedVersionWeights> for OutdatedVersionWeights {
        fn from(value: nym_mixnet_contract_common::OutdatedVersionWeights) -> Self {
            OutdatedVersionWeights {
                major: value.major,
                minor: value.minor,
                patch: value.patch,
                prerelease: value.prerelease,
            }
        }
    }

    impl From<nym_mixnet_contract_common::VersionScoreFormulaParams> for VersionScoreFormulaParams {
        fn from(value: nym_mixnet_contract_common::VersionScoreFormulaParams) -> Self {
            VersionScoreFormulaParams {
                penalty: value.penalty.naive_to_f64(),
                penalty_scaling: value.penalty_scaling.naive_to_f64(),
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NodeRefreshBody {
    #[serde(with = "bs58_ed25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub node_identity: ed25519::PublicKey,

    // a poor man's nonce
    pub request_timestamp: i64,

    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    #[schema(value_type = String)]
    pub signature: ed25519::Signature,
}

impl NodeRefreshBody {
    pub fn plaintext(node_identity: ed25519::PublicKey, request_timestamp: i64) -> Vec<u8> {
        node_identity
            .to_bytes()
            .into_iter()
            .chain(request_timestamp.to_be_bytes())
            .chain(b"describe-cache-refresh-request".iter().copied())
            .collect()
    }

    pub fn new(private_key: &ed25519::PrivateKey) -> Self {
        let node_identity = private_key.public_key();
        let request_timestamp = OffsetDateTime::now_utc().unix_timestamp();
        let signature = private_key.sign(Self::plaintext(node_identity, request_timestamp));
        NodeRefreshBody {
            node_identity,
            request_timestamp,
            signature,
        }
    }

    pub fn verify_signature(&self) -> bool {
        self.node_identity
            .verify(
                Self::plaintext(self.node_identity, self.request_timestamp),
                &self.signature,
            )
            .is_ok()
    }

    pub fn is_stale(&self) -> bool {
        let Ok(encoded) = OffsetDateTime::from_unix_timestamp(self.request_timestamp) else {
            return true;
        };
        let now = OffsetDateTime::now_utc();

        if encoded > now {
            return true;
        }

        if (encoded + Duration::from_secs(30)) < now {
            return true;
        }

        false
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UptimeResponse {
    #[schema(value_type = u32)]
    pub mix_id: NodeId,
    // The same as node_performance.last_24h. Legacy
    pub avg_uptime: u8,
    pub node_performance: NodePerformance,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayUptimeResponse {
    pub identity: String,
    // The same as node_performance.last_24h. Legacy
    pub avg_uptime: u8,
    pub node_performance: NodePerformance,
}

type Uptime = u8;

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct MixnodeStatusReportResponse {
    pub mix_id: NodeId,
    pub identity: IdentityKey,
    pub owner: String,
    #[schema(value_type = u8)]
    pub most_recent: Uptime,
    #[schema(value_type = u8)]
    pub last_hour: Uptime,
    #[schema(value_type = u8)]
    pub last_day: Uptime,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct GatewayStatusReportResponse {
    pub identity: String,
    pub owner: String,
    #[schema(value_type = u8)]
    pub most_recent: Uptime,
    #[schema(value_type = u8)]
    pub last_hour: Uptime,
    #[schema(value_type = u8)]
    pub last_day: Uptime,
}

#[derive(Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/PerformanceHistoryResponse.ts"
    )
)]
pub struct PerformanceHistoryResponse {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub history: PaginatedResponse<HistoricalPerformanceResponse>,
}

#[derive(Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/UptimeHistoryResponse.ts"
    )
)]
pub struct UptimeHistoryResponse {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub history: PaginatedResponse<HistoricalUptimeResponse>,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/HistoricalUptimeResponse.ts"
    )
)]
pub struct HistoricalUptimeResponse {
    #[schema(value_type = String, example = "1970-01-01")]
    #[schemars(with = "String")]
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub date: Date,

    pub uptime: Uptime,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/HistoricalPerformanceResponse.ts"
    )
)]
pub struct HistoricalPerformanceResponse {
    #[schema(value_type = String, example = "1970-01-01")]
    #[schemars(with = "String")]
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub date: Date,

    pub performance: f64,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct OldHistoricalUptimeResponse {
    pub date: String,
    #[schema(value_type = u8)]
    pub uptime: Uptime,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct MixnodeUptimeHistoryResponse {
    pub mix_id: NodeId,
    pub identity: String,
    pub history: Vec<OldHistoricalUptimeResponse>,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct GatewayUptimeHistoryResponse {
    pub identity: String,
    pub history: Vec<OldHistoricalUptimeResponse>,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema, ToSchema, Default,
)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/MixnodeStatus.ts"
    )
)]
#[serde(rename_all = "snake_case")]
pub enum MixnodeStatus {
    Active,   // in both the active set and the rewarded set
    Standby,  // only in the rewarded set
    Inactive, // in neither the rewarded set nor the active set, but is bonded
    #[default]
    NotFound, // doesn't even exist in the bonded set
}
impl MixnodeStatus {
    pub fn is_active(&self) -> bool {
        *self == MixnodeStatus::Active
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/MixnodeStatusResponse.ts"
    )
)]
pub struct MixnodeStatusResponse {
    pub status: MixnodeStatus,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct NodePerformance {
    #[schema(value_type = String)]
    pub most_recent: Performance,
    #[schema(value_type = String)]
    pub last_hour: Performance,
    #[schema(value_type = String)]
    pub last_24h: Performance,
}

// imo for now there's no point in exposing more than that,
// nym-api shouldn't be calculating apy or stake saturation for you.
// it should just return its own metrics (performance) and then you can do with it as you wish
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/NodeAnnotationV1.ts"
    )
)]
pub struct NodeAnnotationV1 {
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    // legacy
    #[schema(value_type = String)]
    pub last_24h_performance: Performance,
    pub current_role: Option<DisplayRole>,

    pub detailed_performance: DetailedNodePerformanceV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/ChainInteractionCapabilitiesDetailed.ts"
    )
)]
pub struct ChainInteractionCapabilitiesDetailed {
    #[schema(value_type = CoinSchema)]
    #[cfg_attr(feature = "generate-ts", ts(type = "Coin"))]
    pub on_chain_balance: Coin,

    // later to be expanded with information on whether the grant would cover
    // cosmwasm executemsg, but for now we assume any feegrant is sufficient
    pub is_feegrant_grantee: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/NodeAnnotationV2.ts"
    )
)]
pub struct NodeAnnotationV2 {
    pub current_role: Option<DisplayRole>,

    pub chain_interaction_capabilities: Option<ChainInteractionCapabilitiesDetailed>,

    pub detailed_performance: DetailedNodePerformanceV2,
}

impl From<NodeAnnotationV2> for NodeAnnotationV1 {
    fn from(value: NodeAnnotationV2) -> Self {
        // map it from 0-1 range into 0-100
        let scaled_performance =
            value.detailed_performance.performance_score.clamp(0.0, 1.0) * 100.;
        #[allow(clippy::unwrap_used)]
        let legacy_performance =
            Performance::from_percentage_value(scaled_performance as u64).unwrap();

        NodeAnnotationV1 {
            last_24h_performance: legacy_performance,
            current_role: value.current_role,
            detailed_performance: DetailedNodePerformanceV1 {
                performance_score: value.detailed_performance.performance_score,
                routing_score: value.detailed_performance.routing_score,
                config_score: value.detailed_performance.config_score.into(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DetailedNodePerformanceV1.ts"
    )
)]
#[non_exhaustive]
pub struct DetailedNodePerformanceV1 {
    /// routing_score * config_score
    pub performance_score: f64,

    pub routing_score: RoutingScore,
    pub config_score: ConfigScoreV1,
}

impl DetailedNodePerformanceV1 {
    pub fn new(
        performance_score: f64,
        routing_score: RoutingScore,
        config_score: ConfigScoreV1,
    ) -> DetailedNodePerformanceV1 {
        Self {
            performance_score,
            routing_score,
            config_score,
        }
    }

    pub fn to_rewarding_performance(&self) -> Performance {
        Performance::naive_try_from_f64(self.performance_score).unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DetailedNodePerformanceV2.ts"
    )
)]
#[non_exhaustive]
pub struct DetailedNodePerformanceV2 {
    /// routing_score * config_score
    /// or
    /// routing_score * config_score * stress_testing_score, if enabled
    pub performance_score: f64,

    pub routing_score: RoutingScore,
    pub config_score: ConfigScoreV2,
    pub stress_testing_score: StressTestingScore,
}

impl DetailedNodePerformanceV2 {
    pub fn new(
        performance_score: f64,
        routing_score: RoutingScore,
        config_score: ConfigScoreV2,
        stress_testing_score: StressTestingScore,
    ) -> DetailedNodePerformanceV2 {
        Self {
            performance_score,
            routing_score,
            config_score,
            stress_testing_score,
        }
    }

    pub fn to_rewarding_performance(&self) -> Performance {
        Performance::naive_try_from_f64(self.performance_score).unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/RoutingScore.ts")
)]
#[non_exhaustive]
pub struct RoutingScore {
    /// Total score after taking all the criteria into consideration
    pub score: f64,
}

impl RoutingScore {
    pub fn new(score: f64) -> RoutingScore {
        Self { score }
    }

    pub const fn zero() -> RoutingScore {
        RoutingScore { score: 0.0 }
    }

    pub fn legacy_performance(&self) -> Performance {
        Performance::naive_try_from_f64(self.score).unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/StressTestingScore.ts"
    )
)]
pub struct StressTestingScore {
    pub score: f64,
    /// Distinguishes a genuine zero score (node was tested and scored 0) from
    /// "node was unreachable" (no successful sample was collected). Consumers may use
    /// this to decide whether to penalise the node or treat the score as missing.
    pub was_reachable: bool,
}

impl StressTestingScore {
    pub fn unreachable() -> Self {
        StressTestingScore {
            score: 0.0,
            was_reachable: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/ConfigScoreV2.ts"
    )
)]
#[non_exhaustive]
pub struct ConfigScoreV2 {
    /// Total score after taking all the criteria into consideration
    pub score: f64,

    pub versions_behind: Option<u32>,
    pub self_described_api_available: bool,
    pub accepted_terms_and_conditions: bool,
    pub runs_nym_node_binary: bool,

    /// Describes the node is capable of sending chain/contract transactions
    pub chain_interaction_capabilities: ChainInteractionCapabilities,
}

impl ConfigScoreV2 {
    pub fn new(
        score: f64,
        versions_behind: u32,
        accepted_terms_and_conditions: bool,
        runs_nym_node_binary: bool,
        chain_interaction_capabilities: ChainInteractionCapabilities,
    ) -> ConfigScoreV2 {
        Self {
            score,
            versions_behind: Some(versions_behind),
            self_described_api_available: true,
            accepted_terms_and_conditions,
            runs_nym_node_binary,
            chain_interaction_capabilities,
        }
    }

    pub fn bad_semver() -> ConfigScoreV2 {
        ConfigScoreV2 {
            score: 0.0,
            versions_behind: None,
            self_described_api_available: true,
            accepted_terms_and_conditions: false,
            runs_nym_node_binary: false,
            chain_interaction_capabilities: Default::default(),
        }
    }

    pub fn unavailable() -> ConfigScoreV2 {
        ConfigScoreV2 {
            score: 0.0,
            versions_behind: None,
            self_described_api_available: false,
            accepted_terms_and_conditions: false,
            runs_nym_node_binary: false,
            chain_interaction_capabilities: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/ChainInteractionCapabilities.ts"
    )
)]
pub struct ChainInteractionCapabilities {
    pub has_sufficient_tokens: bool,
    pub is_fee_grant_grantee: bool,
}

impl ChainInteractionCapabilities {
    pub fn new(has_sufficient_tokens: bool, is_fee_grant_grantee: bool) -> Self {
        Self {
            has_sufficient_tokens,
            is_fee_grant_grantee,
        }
    }

    pub fn can_send_transactions(&self) -> bool {
        self.has_sufficient_tokens || self.is_fee_grant_grantee
    }
}

impl From<ConfigScoreV2> for ConfigScoreV1 {
    fn from(score_v2: ConfigScoreV2) -> ConfigScoreV1 {
        ConfigScoreV1 {
            score: score_v2.score,
            versions_behind: score_v2.versions_behind,
            self_described_api_available: score_v2.self_described_api_available,
            accepted_terms_and_conditions: score_v2.accepted_terms_and_conditions,
            runs_nym_node_binary: score_v2.runs_nym_node_binary,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/ConfigScoreV1.ts"
    )
)]
#[non_exhaustive]
pub struct ConfigScoreV1 {
    /// Total score after taking all the criteria into consideration
    pub score: f64,

    pub versions_behind: Option<u32>,
    pub self_described_api_available: bool,
    pub accepted_terms_and_conditions: bool,
    pub runs_nym_node_binary: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/AnnotationResponseV1.ts"
    )
)]
pub struct AnnotationResponseV1 {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub annotation: Option<NodeAnnotationV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/AnnotationResponseV2.ts"
    )
)]
pub struct AnnotationResponseV2 {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub annotation: Option<NodeAnnotationV2>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/NodePerformanceResponse.ts"
    )
)]
pub struct NodePerformanceResponse {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub performance: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/NodeDatePerformanceResponse.ts"
    )
)]
pub struct NodeDatePerformanceResponse {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    #[schema(value_type = String, example = "1970-01-01")]
    #[schemars(with = "String")]
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub date: Date,
    pub performance: Option<f64>,
}
