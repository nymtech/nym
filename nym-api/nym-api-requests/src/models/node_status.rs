// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::PlaceholderJsonSchemaImpl;
use crate::pagination::PaginatedResponse;
use cosmwasm_std::Decimal;
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

use crate::models::DisplayRole;
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
    pub owner: String,
    pub history: Vec<OldHistoricalUptimeResponse>,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct GatewayUptimeHistoryResponse {
    pub identity: String,
    pub owner: String,
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
        export_to = "ts-packages/types/src/types/rust/NodeAnnotation.ts"
    )
)]
pub struct NodeAnnotation {
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    // legacy
    #[schema(value_type = String)]
    pub last_24h_performance: Performance,
    pub current_role: Option<DisplayRole>,

    pub detailed_performance: DetailedNodePerformance,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DetailedNodePerformance.ts"
    )
)]
#[non_exhaustive]
pub struct DetailedNodePerformance {
    /// routing_score * config_score
    pub performance_score: f64,

    pub routing_score: RoutingScore,
    pub config_score: ConfigScore,
}

impl DetailedNodePerformance {
    pub fn new(
        performance_score: f64,
        routing_score: RoutingScore,
        config_score: ConfigScore,
    ) -> DetailedNodePerformance {
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
    ts(export, export_to = "ts-packages/types/src/types/rust/ConfigScore.ts")
)]
#[non_exhaustive]
pub struct ConfigScore {
    /// Total score after taking all the criteria into consideration
    pub score: f64,

    pub versions_behind: Option<u32>,
    pub self_described_api_available: bool,
    pub accepted_terms_and_conditions: bool,
    pub runs_nym_node_binary: bool,
}

impl ConfigScore {
    pub fn new(
        score: f64,
        versions_behind: u32,
        accepted_terms_and_conditions: bool,
        runs_nym_node_binary: bool,
    ) -> ConfigScore {
        Self {
            score,
            versions_behind: Some(versions_behind),
            self_described_api_available: true,
            accepted_terms_and_conditions,
            runs_nym_node_binary,
        }
    }

    pub fn bad_semver() -> ConfigScore {
        ConfigScore {
            score: 0.0,
            versions_behind: None,
            self_described_api_available: true,
            accepted_terms_and_conditions: false,
            runs_nym_node_binary: false,
        }
    }

    pub fn unavailable() -> ConfigScore {
        ConfigScore {
            score: 0.0,
            versions_behind: None,
            self_described_api_available: false,
            accepted_terms_and_conditions: false,
            runs_nym_node_binary: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/AnnotationResponse.ts"
    )
)]
pub struct AnnotationResponse {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub annotation: Option<NodeAnnotation>,
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
