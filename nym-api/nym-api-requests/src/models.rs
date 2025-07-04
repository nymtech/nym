// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![allow(deprecated)]

use crate::helpers::unix_epoch;
use crate::helpers::PlaceholderJsonSchemaImpl;
use crate::legacy::{
    LegacyGatewayBondWithId, LegacyMixNodeBondWithLayer, LegacyMixNodeDetailsWithLayer,
};
use crate::nym_nodes::SemiSkimmedNode;
use crate::nym_nodes::{BasicEntryInformation, NodeRole, SkimmedNode};
use crate::pagination::PaginatedResponse;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use nym_contracts_common::NaiveFloat;
use nym_crypto::asymmetric::ed25519::{self, serde_helpers::bs58_ed25519_pubkey};
use nym_crypto::asymmetric::x25519::{self, serde_helpers::bs58_x25519_pubkey};
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::reward_params::{Performance, RewardingParams};
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::{GatewayBond, IdentityKey, Interval, MixNode, NodeId, Percent};
use nym_network_defaults::{DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT};
use nym_node_requests::api::v1::authenticator::models::Authenticator;
use nym_node_requests::api::v1::gateway::models::Wireguard;
use nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter;
use nym_node_requests::api::v1::node::models::{AuxiliaryDetails, NodeRoles};
use nym_noise_keys::VersionedNoiseKey;
use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};
use std::net::IpAddr;
use std::ops::{Deref, DerefMut};
use std::{fmt, time::Duration};
use thiserror::Error;
use time::{Date, OffsetDateTime};
use tracing::{error, warn};
use utoipa::{IntoParams, ToResponse, ToSchema};

pub use nym_mixnet_contract_common::{EpochId, KeyRotationId, KeyRotationState};
pub use nym_node_requests::api::v1::node::models::BinaryBuildInformationOwned;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct RequestError {
    message: String,
}

impl RequestError {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        RequestError {
            message: msg.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn empty() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl Display for RequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.message, f)
    }
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/DisplayRole.ts")
)]
pub enum DisplayRole {
    EntryGateway,
    Layer1,
    Layer2,
    Layer3,
    ExitGateway,
    Standby,
}

impl From<Role> for DisplayRole {
    fn from(role: Role) -> Self {
        match role {
            Role::EntryGateway => DisplayRole::EntryGateway,
            Role::Layer1 => DisplayRole::Layer1,
            Role::Layer2 => DisplayRole::Layer2,
            Role::Layer3 => DisplayRole::Layer3,
            Role::ExitGateway => DisplayRole::ExitGateway,
            Role::Standby => DisplayRole::Standby,
        }
    }
}

impl From<DisplayRole> for Role {
    fn from(role: DisplayRole) -> Self {
        match role {
            DisplayRole::EntryGateway => Role::EntryGateway,
            DisplayRole::Layer1 => Role::Layer1,
            DisplayRole::Layer2 => Role::Layer2,
            DisplayRole::Layer3 => Role::Layer3,
            DisplayRole::ExitGateway => Role::ExitGateway,
            DisplayRole::Standby => Role::Standby,
        }
    }
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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[schema(title = "LegacyMixNodeDetailsWithLayer")]
pub struct LegacyMixNodeDetailsWithLayerSchema {
    /// Basic bond information of this mixnode, such as owner address, original pledge, etc.
    #[schema(example = "unimplemented schema")]
    pub bond_information: String,

    /// Details used for computation of rewarding related data.
    #[schema(example = "unimplemented schema")]
    pub rewarding_details: String,

    /// Adjustments to the mixnode that are ought to happen during future epoch transitions.
    #[schema(example = "unimplemented schema")]
    pub pending_changes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MixNodeBondAnnotated {
    #[schema(value_type = LegacyMixNodeDetailsWithLayerSchema)]
    pub mixnode_details: LegacyMixNodeDetailsWithLayer,
    #[schema(value_type = String)]
    pub stake_saturation: StakeSaturation,
    #[schema(value_type = String)]
    pub uncapped_stake_saturation: StakeSaturation,
    // NOTE: the performance field is deprecated in favour of node_performance
    #[schema(value_type = String)]
    pub performance: Performance,
    pub node_performance: NodePerformance,
    #[schema(value_type = String)]
    pub estimated_operator_apy: Decimal,
    #[schema(value_type = String)]
    pub estimated_delegators_apy: Decimal,
    pub blacklisted: bool,

    // a rather temporary thing until we query self-described endpoints of mixnodes
    #[serde(default)]
    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,
}

#[derive(Debug, Error)]
pub enum MalformedNodeBond {
    #[error("the associated ed25519 identity key is malformed")]
    InvalidEd25519Key,

    #[error("the associated x25519 sphinx key is malformed")]
    InvalidX25519Key,
}

impl MixNodeBondAnnotated {
    pub fn mix_node(&self) -> &MixNode {
        &self.mixnode_details.bond_information.mix_node
    }

    pub fn mix_id(&self) -> NodeId {
        self.mixnode_details.mix_id()
    }

    pub fn identity_key(&self) -> &str {
        self.mixnode_details.bond_information.identity()
    }

    pub fn owner(&self) -> &Addr {
        self.mixnode_details.bond_information.owner()
    }

    pub fn version(&self) -> &str {
        &self.mixnode_details.bond_information.mix_node.version
    }

    pub fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        Ok(SkimmedNode {
            node_id: self.mix_id(),
            ed25519_identity_pubkey: self
                .identity_key()
                .parse()
                .map_err(|_| MalformedNodeBond::InvalidEd25519Key)?,
            ip_addresses: self.ip_addresses.clone(),
            mix_port: self.mix_node().mix_port,
            x25519_sphinx_pubkey: self
                .mix_node()
                .sphinx_key
                .parse()
                .map_err(|_| MalformedNodeBond::InvalidX25519Key)?,
            role,
            supported_roles: DeclaredRoles {
                mixnode: true,
                entry: false,
                exit_nr: false,
                exit_ipr: false,
            },
            entry: None,
            performance: self.node_performance.last_24h,
        })
    }

    pub fn try_to_semi_skimmed_node(
        &self,
        role: NodeRole,
    ) -> Result<SemiSkimmedNode, MalformedNodeBond> {
        let skimmed_node = self.try_to_skimmed_node(role)?;
        Ok(SemiSkimmedNode {
            basic: skimmed_node,
            x25519_noise_versioned_key: None, // legacy node won't ever support Noise
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayBondAnnotated {
    pub gateway_bond: LegacyGatewayBondWithId,

    #[serde(default)]
    pub self_described: Option<GatewayDescription>,

    // NOTE: the performance field is deprecated in favour of node_performance
    #[schema(value_type = String)]
    pub performance: Performance,
    pub node_performance: NodePerformance,
    pub blacklisted: bool,

    #[serde(default)]
    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,
}

impl GatewayBondAnnotated {
    pub fn version(&self) -> &str {
        &self.gateway_bond.gateway.version
    }

    pub fn identity(&self) -> &String {
        self.gateway_bond.bond.identity()
    }

    pub fn owner(&self) -> &Addr {
        self.gateway_bond.bond.owner()
    }

    pub fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        Ok(SkimmedNode {
            node_id: self.gateway_bond.node_id,
            ip_addresses: self.ip_addresses.clone(),
            ed25519_identity_pubkey: self
                .gateway_bond
                .gateway
                .identity_key
                .parse()
                .map_err(|_| MalformedNodeBond::InvalidEd25519Key)?,
            mix_port: self.gateway_bond.bond.gateway.mix_port,
            x25519_sphinx_pubkey: self
                .gateway_bond
                .gateway
                .sphinx_key
                .parse()
                .map_err(|_| MalformedNodeBond::InvalidX25519Key)?,
            role,
            supported_roles: DeclaredRoles {
                mixnode: false,
                entry: true,
                exit_nr: false,
                exit_ipr: false,
            },
            entry: Some(BasicEntryInformation {
                hostname: None,
                ws_port: self.gateway_bond.bond.gateway.clients_port,
                wss_port: None,
            }),
            performance: self.node_performance.last_24h,
        })
    }

    pub fn try_to_semi_skimmed_node(
        &self,
        role: NodeRole,
    ) -> Result<SemiSkimmedNode, MalformedNodeBond> {
        let skimmed_node = self.try_to_skimmed_node(role)?;
        Ok(SemiSkimmedNode {
            basic: skimmed_node,
            x25519_noise_versioned_key: None, // legacy node won't ever support Noise
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayDescription {
    // for now only expose what we need. this struct will evolve in the future (or be incorporated into nym-node properly)
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, ToSchema, IntoParams)]
pub struct ComputeRewardEstParam {
    #[schema(value_type = Option<String>)]
    #[param(value_type = Option<String>)]
    pub performance: Option<Performance>,
    pub active_in_rewarded_set: Option<bool>,
    pub pledge_amount: Option<u64>,
    pub total_delegation: Option<u64>,
    #[schema(value_type = Option<CoinSchema>)]
    #[param(value_type = Option<CoinSchema>)]
    pub interval_operating_cost: Option<Coin>,
    #[schema(value_type = Option<String>)]
    #[param(value_type = Option<String>)]
    pub profit_margin_percent: Option<Percent>,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/RewardEstimationResponse.ts"
    )
)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RewardEstimationResponse {
    pub estimation: RewardEstimate,
    pub reward_params: RewardingParams,
    pub epoch: Interval,
    #[cfg_attr(feature = "generate-ts", ts(type = "number"))]
    pub as_at: i64,
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

pub type StakeSaturation = Decimal;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/SelectionChance.ts"
    )
)]
#[deprecated]
pub enum SelectionChance {
    High,
    Good,
    Low,
}

impl From<f64> for SelectionChance {
    fn from(p: f64) -> SelectionChance {
        match p {
            p if p >= 0.7 => SelectionChance::High,
            p if p >= 0.3 => SelectionChance::Good,
            _ => SelectionChance::Low,
        }
    }
}

impl From<Decimal> for SelectionChance {
    fn from(p: Decimal) -> Self {
        match p {
            p if p >= Decimal::from_ratio(70u32, 100u32) => SelectionChance::High,
            p if p >= Decimal::from_ratio(30u32, 100u32) => SelectionChance::Good,
            _ => SelectionChance::Low,
        }
    }
}

impl fmt::Display for SelectionChance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectionChance::High => write!(f, "High"),
            SelectionChance::Good => write!(f, "Good"),
            SelectionChance::Low => write!(f, "Low"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/InclusionProbabilityResponse.ts"
    )
)]
#[deprecated]
pub struct InclusionProbabilityResponse {
    pub in_active: SelectionChance,
    pub in_reserve: SelectionChance,
}

impl fmt::Display for InclusionProbabilityResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "in_active: {}, in_reserve: {}",
            self.in_active, self.in_reserve
        )
    }
}

#[deprecated]
#[derive(Clone, Serialize, schemars::JsonSchema, ToSchema)]
pub struct AllInclusionProbabilitiesResponse {
    pub inclusion_probabilities: Vec<InclusionProbability>,
    pub samples: u64,
    pub elapsed: Duration,
    pub delta_max: f64,
    pub delta_l2: f64,
    pub as_at: i64,
}

#[deprecated]
#[derive(Clone, Serialize, schemars::JsonSchema, ToSchema)]
pub struct InclusionProbability {
    #[schema(value_type = u32)]
    pub mix_id: NodeId,
    pub in_active: f64,
    pub in_reserve: f64,
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

#[derive(ToSchema)]
#[schema(title = "Coin")]
pub struct CoinSchema {
    pub denom: String,
    #[schema(value_type = String)]
    pub amount: Uint128,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema, ToResponse)]
pub struct CirculatingSupplyResponse {
    #[schema(value_type = CoinSchema)]
    pub total_supply: Coin,
    #[schema(value_type = CoinSchema)]
    pub mixmining_reserve: Coin,
    #[schema(value_type = CoinSchema)]
    pub vesting_tokens: Coin,
    #[schema(value_type = CoinSchema)]
    pub circulating_supply: Coin,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct HostInformation {
    #[schema(value_type = Vec<String>)]
    pub ip_address: Vec<IpAddr>,
    pub hostname: Option<String>,
    pub keys: HostKeys,
}

impl From<nym_node_requests::api::v1::node::models::HostInformation> for HostInformation {
    fn from(value: nym_node_requests::api::v1::node::models::HostInformation) -> Self {
        HostInformation {
            ip_address: value.ip_address,
            hostname: value.hostname,
            keys: value.keys.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct HostKeys {
    #[serde(with = "bs58_ed25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub ed25519: ed25519::PublicKey,

    #[deprecated(note = "use the current_x25519_sphinx_key with explicit rotation information")]
    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub x25519: x25519::PublicKey,

    pub current_x25519_sphinx_key: SphinxKey,

    #[serde(default)]
    pub pre_announced_x25519_sphinx_key: Option<SphinxKey>,

    #[serde(default)]
    pub x25519_versioned_noise: Option<VersionedNoiseKey>,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct SphinxKey {
    pub rotation_id: u32,

    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub public_key: x25519::PublicKey,
}

impl From<nym_node_requests::api::v1::node::models::SphinxKey> for SphinxKey {
    fn from(value: nym_node_requests::api::v1::node::models::SphinxKey) -> Self {
        SphinxKey {
            rotation_id: value.rotation_id,
            public_key: value.public_key,
        }
    }
}

impl From<nym_node_requests::api::v1::node::models::HostKeys> for HostKeys {
    fn from(value: nym_node_requests::api::v1::node::models::HostKeys) -> Self {
        HostKeys {
            ed25519: value.ed25519_identity,
            x25519: value.x25519_sphinx,
            current_x25519_sphinx_key: value.primary_x25519_sphinx_key.into(),
            pre_announced_x25519_sphinx_key: value.pre_announced_x25519_sphinx_key.map(Into::into),
            x25519_versioned_noise: value.x25519_versioned_noise,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct WebSockets {
    pub ws_port: u16,

    pub wss_port: Option<u16>,
}

impl From<nym_node_requests::api::v1::gateway::models::WebSockets> for WebSockets {
    fn from(value: nym_node_requests::api::v1::gateway::models::WebSockets) -> Self {
        WebSockets {
            ws_port: value.ws_port,
            wss_port: value.wss_port,
        }
    }
}

pub fn de_rfc3339_or_default<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(time::serde::rfc3339::deserialize(deserializer).unwrap_or_else(|_| unix_epoch()))
}

// for all intents and purposes it's just OffsetDateTime, but we need JsonSchema...
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct OffsetDateTimeJsonSchemaWrapper(
    #[serde(
        default = "unix_epoch",
        with = "crate::helpers::overengineered_offset_date_time_serde"
    )]
    #[schema(inline)]
    pub OffsetDateTime,
);

impl Default for OffsetDateTimeJsonSchemaWrapper {
    fn default() -> Self {
        OffsetDateTimeJsonSchemaWrapper(unix_epoch())
    }
}

impl Display for OffsetDateTimeJsonSchemaWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<OffsetDateTimeJsonSchemaWrapper> for OffsetDateTime {
    fn from(value: OffsetDateTimeJsonSchemaWrapper) -> Self {
        value.0
    }
}

impl From<OffsetDateTime> for OffsetDateTimeJsonSchemaWrapper {
    fn from(value: OffsetDateTime) -> Self {
        OffsetDateTimeJsonSchemaWrapper(value)
    }
}

impl Deref for OffsetDateTimeJsonSchemaWrapper {
    type Target = OffsetDateTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OffsetDateTimeJsonSchemaWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// implementation taken from: https://github.com/GREsau/schemars/pull/207
impl JsonSchema for OffsetDateTimeJsonSchemaWrapper {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        "DateTime".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            format: Some("date-time".into()),
            ..Default::default()
        }
        .into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NymNodeDescription {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub contract_node_type: DescribedNodeType,
    pub description: NymNodeData,
}

impl NymNodeDescription {
    pub fn version(&self) -> &str {
        &self.description.build_information.build_version
    }

    pub fn entry_information(&self) -> BasicEntryInformation {
        BasicEntryInformation {
            hostname: self.description.host_information.hostname.clone(),
            ws_port: self.description.mixnet_websockets.ws_port,
            wss_port: self.description.mixnet_websockets.wss_port,
        }
    }

    pub fn ed25519_identity_key(&self) -> ed25519::PublicKey {
        self.description.host_information.keys.ed25519
    }

    pub fn current_sphinx_key(&self, current_rotation_id: u32) -> x25519::PublicKey {
        let keys = &self.description.host_information.keys;

        if keys.current_x25519_sphinx_key.rotation_id == u32::MAX {
            // legacy case (i.e. node doesn't support rotation)
            return keys.current_x25519_sphinx_key.public_key;
        }

        if current_rotation_id == keys.current_x25519_sphinx_key.rotation_id {
            // it's the 'current' key
            return keys.current_x25519_sphinx_key.public_key;
        }

        if let Some(pre_announced) = &keys.pre_announced_x25519_sphinx_key {
            if pre_announced.rotation_id == current_rotation_id {
                return pre_announced.public_key;
            }
        }

        warn!(
            "unexpected key rotation {current_rotation_id} for node {}",
            self.node_id
        );
        // this should never be reached, but just in case, return the fallback option
        keys.current_x25519_sphinx_key.public_key
    }

    pub fn to_skimmed_node(
        &self,
        current_rotation_id: u32,
        role: NodeRole,
        performance: Performance,
    ) -> SkimmedNode {
        let keys = &self.description.host_information.keys;
        let entry = if self.description.declared_role.entry {
            Some(self.entry_information())
        } else {
            None
        };

        SkimmedNode {
            node_id: self.node_id,
            ed25519_identity_pubkey: keys.ed25519,
            ip_addresses: self.description.host_information.ip_address.clone(),
            mix_port: self.description.mix_port(),
            x25519_sphinx_pubkey: self.current_sphinx_key(current_rotation_id),
            // we can't use the declared roles, we have to take whatever was provided in the contract.
            // why? say this node COULD operate as an exit, but it might be the case the contract decided
            // to assign it an ENTRY role only. we have to use that one instead.
            role,
            supported_roles: self.description.declared_role,
            entry,
            performance,
        }
    }

    pub fn to_semi_skimmed_node(
        &self,
        current_rotation_id: u32,
        role: NodeRole,
        performance: Performance,
    ) -> SemiSkimmedNode {
        let skimmed_node = self.to_skimmed_node(current_rotation_id, role, performance);

        SemiSkimmedNode {
            basic: skimmed_node,
            x25519_noise_versioned_key: self
                .description
                .host_information
                .keys
                .x25519_versioned_noise,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DescribedNodeType.ts"
    )
)]
pub enum DescribedNodeType {
    LegacyMixnode,
    LegacyGateway,
    NymNode,
}

impl DescribedNodeType {
    pub fn is_nym_node(&self) -> bool {
        matches!(self, DescribedNodeType::NymNode)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DeclaredRoles.ts"
    )
)]
pub struct DeclaredRoles {
    pub mixnode: bool,
    pub entry: bool,
    pub exit_nr: bool,
    pub exit_ipr: bool,
}

impl DeclaredRoles {
    pub fn can_operate_exit_gateway(&self) -> bool {
        self.exit_ipr && self.exit_nr
    }
}

impl From<NodeRoles> for DeclaredRoles {
    fn from(value: NodeRoles) -> Self {
        DeclaredRoles {
            mixnode: value.mixnode_enabled,
            entry: value.gateway_enabled,
            exit_nr: value.gateway_enabled && value.network_requester_enabled,
            exit_ipr: value.gateway_enabled && value.ip_packet_router_enabled,
        }
    }
}

// this struct is getting quite bloated...
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NymNodeData {
    #[serde(default)]
    pub last_polled: OffsetDateTimeJsonSchemaWrapper,

    pub host_information: HostInformation,

    #[serde(default)]
    pub declared_role: DeclaredRoles,

    #[serde(default)]
    pub auxiliary_details: AuxiliaryDetails,

    // TODO: do we really care about ALL build info or just the version?
    pub build_information: BinaryBuildInformationOwned,

    #[serde(default)]
    pub network_requester: Option<NetworkRequesterDetails>,

    #[serde(default)]
    pub ip_packet_router: Option<IpPacketRouterDetails>,

    #[serde(default)]
    pub authenticator: Option<AuthenticatorDetails>,

    #[serde(default)]
    pub wireguard: Option<WireguardDetails>,

    // for now we only care about their ws/wss situation, nothing more
    pub mixnet_websockets: WebSockets,
}

impl NymNodeData {
    pub fn mix_port(&self) -> u16 {
        self.auxiliary_details
            .announce_ports
            .mix_port
            .unwrap_or(DEFAULT_MIX_LISTENING_PORT)
    }

    pub fn verloc_port(&self) -> u16 {
        self.auxiliary_details
            .announce_ports
            .verloc_port
            .unwrap_or(DEFAULT_VERLOC_LISTENING_PORT)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct LegacyDescribedGateway {
    pub bond: GatewayBond,
    pub self_described: Option<NymNodeData>,
}

impl From<LegacyGatewayBondWithId> for LegacyDescribedGateway {
    fn from(bond: LegacyGatewayBondWithId) -> Self {
        LegacyDescribedGateway {
            bond: bond.bond,
            self_described: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct LegacyDescribedMixNode {
    pub bond: LegacyMixNodeBondWithLayer,
    pub self_described: Option<NymNodeData>,
}

impl From<LegacyMixNodeBondWithLayer> for LegacyDescribedMixNode {
    fn from(bond: LegacyMixNodeBondWithLayer) -> Self {
        LegacyDescribedMixNode {
            bond,
            self_described: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NetworkRequesterDetails {
    /// address of the embedded network requester
    pub address: String,

    /// flag indicating whether this network requester uses the exit policy rather than the deprecated allow list
    pub uses_exit_policy: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct IpPacketRouterDetails {
    /// address of the embedded ip packet router
    pub address: String,
}

// works for current simple case.
impl From<IpPacketRouter> for IpPacketRouterDetails {
    fn from(value: IpPacketRouter) -> Self {
        IpPacketRouterDetails {
            address: value.address,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct AuthenticatorDetails {
    /// address of the embedded authenticator
    pub address: String,
}

// works for current simple case.
impl From<Authenticator> for AuthenticatorDetails {
    fn from(value: Authenticator) -> Self {
        AuthenticatorDetails {
            address: value.address,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct WireguardDetails {
    pub port: u16,
    pub public_key: String,
}

// works for current simple case.
impl From<Wireguard> for WireguardDetails {
    fn from(value: Wireguard) -> Self {
        WireguardDetails {
            port: value.port,
            public_key: value.public_key,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct ApiHealthResponse {
    pub status: ApiStatus,
    #[serde(default)]
    pub chain_status: ChainStatus,
    pub uptime: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApiStatus {
    Up,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, schemars::JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChainStatus {
    Synced,
    #[default]
    Unknown,
    Stalled {
        #[serde(
            serialize_with = "humantime_serde::serialize",
            deserialize_with = "humantime_serde::deserialize"
        )]
        approximate_amount: Duration,
    },
}

impl ApiHealthResponse {
    pub fn new_healthy(uptime: Duration) -> Self {
        ApiHealthResponse {
            status: ApiStatus::Up,
            chain_status: ChainStatus::Synced,
            uptime: uptime.as_secs(),
        }
    }
}

impl ApiStatus {
    pub fn is_up(&self) -> bool {
        matches!(self, ApiStatus::Up)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct SignerInformationResponse {
    pub cosmos_address: String,

    pub identity: String,

    pub announce_address: String,

    pub verification_key: Option<String>,
}

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

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NoiseDetails {
    pub key: VersionedNoiseKey,

    pub mixnet_port: u16,

    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct KeyRotationInfoResponse {
    #[serde(flatten)]
    pub details: KeyRotationDetails,

    // helper field that holds calculated data based on the `details` field
    // this is to expose the information in a format more easily accessible by humans
    // without having to do any calculations
    pub progress: KeyRotationProgressInfo,
}

impl From<KeyRotationDetails> for KeyRotationInfoResponse {
    fn from(details: KeyRotationDetails) -> Self {
        KeyRotationInfoResponse {
            details,
            progress: KeyRotationProgressInfo {
                current_key_rotation_id: details.current_key_rotation_id(),
                current_rotation_starting_epoch: details.current_rotation_starting_epoch_id(),
                current_rotation_ending_epoch: details.current_rotation_starting_epoch_id()
                    + details.key_rotation_state.validity_epochs
                    - 1,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct KeyRotationProgressInfo {
    pub current_key_rotation_id: u32,

    pub current_rotation_starting_epoch: u32,

    pub current_rotation_ending_epoch: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct KeyRotationDetails {
    pub key_rotation_state: KeyRotationState,

    #[schema(value_type = u32)]
    pub current_absolute_epoch_id: EpochId,

    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub current_epoch_start: OffsetDateTime,

    pub epoch_duration: Duration,
}

impl KeyRotationDetails {
    pub fn current_key_rotation_id(&self) -> u32 {
        self.key_rotation_state
            .key_rotation_id(self.current_absolute_epoch_id)
    }

    pub fn next_rotation_starting_epoch_id(&self) -> EpochId {
        self.key_rotation_state
            .next_rotation_starting_epoch_id(self.current_absolute_epoch_id)
    }

    pub fn current_rotation_starting_epoch_id(&self) -> EpochId {
        self.key_rotation_state
            .current_rotation_starting_epoch_id(self.current_absolute_epoch_id)
    }

    fn current_epoch_progress(&self, now: OffsetDateTime) -> f32 {
        let elapsed = (now - self.current_epoch_start).as_seconds_f32();
        elapsed / self.epoch_duration.as_secs_f32()
    }

    pub fn is_epoch_stuck(&self) -> bool {
        let now = OffsetDateTime::now_utc();
        let progress = self.current_epoch_progress(now);
        if progress > 1. {
            let into_next = 1. - progress;
            // if epoch hasn't progressed for more than 20% of its duration, mark is as stuck
            if into_next > 0.2 {
                let diff_time =
                    Duration::from_secs_f32(into_next * self.epoch_duration.as_secs_f32());
                let expected_epoch_end = self.current_epoch_start + self.epoch_duration;
                warn!("the current epoch is expected to have been over by {expected_epoch_end}. it's already {} overdue!", humantime_serde::re::humantime::format_duration(diff_time));
                return true;
            }
        }

        false
    }

    // based on the current **TIME**, determine what's the expected current rotation id
    pub fn expected_current_rotation_id(&self) -> KeyRotationId {
        let now = OffsetDateTime::now_utc();
        let current_end = now + self.epoch_duration;
        if now < current_end {
            return self
                .key_rotation_state
                .key_rotation_id(self.current_absolute_epoch_id);
        }

        let diff = now - current_end;
        let passed_epochs = diff / self.epoch_duration;
        let expected_current_epoch = self.current_absolute_epoch_id + passed_epochs.floor() as u32;

        self.key_rotation_state
            .key_rotation_id(expected_current_epoch)
    }

    pub fn until_next_rotation(&self) -> Option<Duration> {
        let current_epoch_progress = self.current_epoch_progress(OffsetDateTime::now_utc());
        if current_epoch_progress > 1. {
            return None;
        }

        let next_rotation_epoch = self.next_rotation_starting_epoch_id();
        let full_remaining =
            (next_rotation_epoch - self.current_absolute_epoch_id).checked_add(1)?;

        let epochs_until_next_rotation = (1. - current_epoch_progress) + full_remaining as f32;

        Some(Duration::from_secs_f32(
            epochs_until_next_rotation * self.epoch_duration.as_secs_f32(),
        ))
    }

    pub fn epoch_start_time(&self, absolute_epoch_id: EpochId) -> OffsetDateTime {
        match absolute_epoch_id.cmp(&self.current_absolute_epoch_id) {
            Ordering::Less => {
                let diff = self.current_absolute_epoch_id - absolute_epoch_id;
                self.current_epoch_start - diff * self.epoch_duration
            }
            Ordering::Equal => self.current_epoch_start,
            Ordering::Greater => {
                let diff = absolute_epoch_id - self.current_absolute_epoch_id;
                self.current_epoch_start + diff * self.epoch_duration
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct RewardedSetResponse {
    #[serde(default)]
    #[schema(value_type = u32)]
    pub epoch_id: EpochId,

    pub entry_gateways: Vec<NodeId>,

    pub exit_gateways: Vec<NodeId>,

    pub layer1: Vec<NodeId>,

    pub layer2: Vec<NodeId>,

    pub layer3: Vec<NodeId>,

    pub standby: Vec<NodeId>,
}

impl From<RewardedSetResponse> for nym_mixnet_contract_common::EpochRewardedSet {
    fn from(res: RewardedSetResponse) -> Self {
        nym_mixnet_contract_common::EpochRewardedSet {
            epoch_id: res.epoch_id,
            assignment: nym_mixnet_contract_common::RewardedSet {
                entry_gateways: res.entry_gateways,
                exit_gateways: res.exit_gateways,
                layer1: res.layer1,
                layer2: res.layer2,
                layer3: res.layer3,
                standby: res.standby,
            },
        }
    }
}

impl From<nym_mixnet_contract_common::EpochRewardedSet> for RewardedSetResponse {
    fn from(r: nym_mixnet_contract_common::EpochRewardedSet) -> Self {
        RewardedSetResponse {
            epoch_id: r.epoch_id,
            entry_gateways: r.assignment.entry_gateways,
            exit_gateways: r.assignment.exit_gateways,
            layer1: r.assignment.layer1,
            layer2: r.assignment.layer2,
            layer3: r.assignment.layer3,
            standby: r.assignment.standby,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ChainStatusResponse {
    pub connected_nyxd: String,
    pub status: DetailedChainStatus,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DetailedChainStatus {
    pub abci: crate::models::tendermint_types::AbciInfo,
    pub latest_block: BlockInfo,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BlockInfo {
    pub block_id: BlockId,
    pub block: FullBlockInfo,
    // if necessary we might put block data here later too
}

impl From<tendermint_rpc::endpoint::block::Response> for BlockInfo {
    fn from(value: tendermint_rpc::endpoint::block::Response) -> Self {
        BlockInfo {
            block_id: value.block_id.into(),
            block: FullBlockInfo {
                header: value.block.header.into(),
            },
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FullBlockInfo {
    pub header: BlockHeader,
}

// copy tendermint types definitions whilst deriving schema types on them and dropping unwanted fields
pub mod tendermint_types {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use tendermint::abci::response::Info;
    use tendermint::block::header::Version;
    use tendermint::{block, Hash};
    use utoipa::ToSchema;

    #[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
    pub struct AbciInfo {
        /// Some arbitrary information.
        pub data: String,

        /// The application software semantic version.
        pub version: String,

        /// The application protocol version.
        pub app_version: u64,

        /// The latest block for which the app has called [`Commit`].
        pub last_block_height: u64,

        /// The latest result of [`Commit`].
        pub last_block_app_hash: String,
    }

    impl From<Info> for AbciInfo {
        fn from(value: Info) -> Self {
            AbciInfo {
                data: value.data,
                version: value.version,
                app_version: value.app_version,
                last_block_height: value.last_block_height.value(),
                last_block_app_hash: value.last_block_app_hash.to_string(),
            }
        }
    }

    /// `Version` contains the protocol version for the blockchain and the
    /// application.
    ///
    /// <https://github.com/tendermint/spec/blob/d46cd7f573a2c6a2399fcab2cde981330aa63f37/spec/core/data_structures.md#version>
    #[derive(
        Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, ToSchema,
    )]
    pub struct HeaderVersion {
        /// Block version
        pub block: u64,

        /// App version
        pub app: u64,
    }

    impl From<tendermint::block::header::Version> for HeaderVersion {
        fn from(value: Version) -> Self {
            HeaderVersion {
                block: value.block,
                app: value.app,
            }
        }
    }

    /// Block identifiers which contain two distinct Merkle roots of the block,
    /// as well as the number of parts in the block.
    ///
    /// <https://github.com/tendermint/spec/blob/d46cd7f573a2c6a2399fcab2cde981330aa63f37/spec/core/data_structures.md#blockid>
    ///
    /// Default implementation is an empty Id as defined by the Go implementation in
    /// <https://github.com/tendermint/tendermint/blob/1635d1339c73ae6a82e062cd2dc7191b029efa14/types/block.go#L1204>.
    ///
    /// If the Hash is empty in BlockId, the BlockId should be empty (encoded to None).
    /// This is implemented outside of this struct. Use the Default trait to check for an empty BlockId.
    /// See: <https://github.com/informalsystems/tendermint-rs/issues/663>
    #[derive(
        Serialize,
        Deserialize,
        Copy,
        Clone,
        Debug,
        Default,
        Hash,
        Eq,
        PartialEq,
        PartialOrd,
        Ord,
        JsonSchema,
        ToSchema,
    )]
    pub struct BlockId {
        /// The block's main hash is the Merkle root of all the fields in the
        /// block header.
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub hash: Hash,

        /// Parts header (if available) is used for secure gossipping of the block
        /// during consensus. It is the Merkle root of the complete serialized block
        /// cut into parts.
        ///
        /// PartSet is used to split a byteslice of data into parts (pieces) for
        /// transmission. By splitting data into smaller parts and computing a
        /// Merkle root hash on the list, you can verify that a part is
        /// legitimately part of the complete data, and the part can be forwarded
        /// to other peers before all the parts are known. In short, it's a fast
        /// way to propagate a large file over a gossip network.
        ///
        /// <https://github.com/tendermint/tendermint/wiki/Block-Structure#partset>
        ///
        /// PartSetHeader in protobuf is defined as never nil using the gogoproto
        /// annotations. This does not translate to Rust, but we can indicate this
        /// in the domain type.
        pub part_set_header: PartSetHeader,
    }

    impl From<block::Id> for BlockId {
        fn from(value: block::Id) -> Self {
            BlockId {
                hash: value.hash,
                part_set_header: value.part_set_header.into(),
            }
        }
    }

    /// Block parts header
    #[derive(
        Clone,
        Copy,
        Debug,
        Default,
        Hash,
        Eq,
        PartialEq,
        PartialOrd,
        Ord,
        Deserialize,
        Serialize,
        JsonSchema,
        ToSchema,
    )]
    #[non_exhaustive]
    pub struct PartSetHeader {
        /// Number of parts in this block
        pub total: u32,

        /// Hash of the parts set header,
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub hash: Hash,
    }

    impl From<tendermint::block::parts::Header> for PartSetHeader {
        fn from(value: block::parts::Header) -> Self {
            PartSetHeader {
                total: value.total,
                hash: value.hash,
            }
        }
    }

    /// Block `Header` values contain metadata about the block and about the
    /// consensus, as well as commitments to the data in the current block, the
    /// previous block, and the results returned by the application.
    ///
    /// <https://github.com/tendermint/spec/blob/d46cd7f573a2c6a2399fcab2cde981330aa63f37/spec/core/data_structures.md#header>
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
    pub struct BlockHeader {
        /// Header version
        pub version: HeaderVersion,

        /// Chain ID
        pub chain_id: String,

        /// Current block height
        pub height: u64,

        /// Current timestamp
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub time: tendermint::Time,

        /// Previous block info
        pub last_block_id: Option<BlockId>,

        /// Commit from validators from the last block
        #[schemars(with = "Option<String>")]
        #[schema(value_type = Option<String>)]
        pub last_commit_hash: Option<Hash>,

        /// Merkle root of transaction hashes
        #[schemars(with = "Option<String>")]
        #[schema(value_type = Option<String>)]
        pub data_hash: Option<Hash>,

        /// Validators for the current block
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub validators_hash: Hash,

        /// Validators for the next block
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub next_validators_hash: Hash,

        /// Consensus params for the current block
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub consensus_hash: Hash,

        /// State after txs from the previous block
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub app_hash: Hash,

        /// Root hash of all results from the txs from the previous block
        #[schemars(with = "Option<String>")]
        #[schema(value_type = Option<String>)]
        pub last_results_hash: Option<Hash>,

        /// Hash of evidence included in the block
        #[schemars(with = "Option<String>")]
        #[schema(value_type = Option<String>)]
        pub evidence_hash: Option<Hash>,

        /// Original proposer of the block
        #[serde(with = "nym_serde_helpers::hex")]
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub proposer_address: Vec<u8>,
    }

    impl From<block::Header> for BlockHeader {
        fn from(value: block::Header) -> Self {
            BlockHeader {
                version: value.version.into(),
                chain_id: value.chain_id.to_string(),
                height: value.height.value(),
                time: value.time,
                last_block_id: value.last_block_id.map(Into::into),
                last_commit_hash: value.last_commit_hash,
                data_hash: value.data_hash,
                validators_hash: value.validators_hash,
                next_validators_hash: value.next_validators_hash,
                consensus_hash: value.consensus_hash,
                app_hash: Hash::try_from(value.app_hash.as_bytes().to_vec()).unwrap_or_default(),
                last_results_hash: value.last_results_hash,
                evidence_hash: value.evidence_hash,
                proposer_address: value.proposer_address.as_bytes().to_vec(),
            }
        }
    }
}

use crate::models::tendermint_types::{BlockHeader, BlockId};
pub use config_score::*;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_date_time_json_schema_wrapper_serde_backwards_compat() {
        let mut dummy = OffsetDateTimeJsonSchemaWrapper::default();
        dummy.0 += Duration::from_millis(1);
        let ser = serde_json::to_string(&dummy).unwrap();

        assert_eq!("\"1970-01-01 00:00:00.001 +00:00:00\"", ser);

        let human_readable = "\"2024-05-23 07:41:02.756283766 +00:00:00\"";
        let rfc3339 = "\"2002-10-02T15:00:00Z\"";
        let rfc3339_offset = "\"2002-10-02T10:00:00-05:00\"";

        let de = serde_json::from_str::<OffsetDateTimeJsonSchemaWrapper>(human_readable).unwrap();
        assert_eq!(de.0.unix_timestamp(), 1716450062);

        let de = serde_json::from_str::<OffsetDateTimeJsonSchemaWrapper>(rfc3339).unwrap();
        assert_eq!(de.0.unix_timestamp(), 1033570800);

        let de = serde_json::from_str::<OffsetDateTimeJsonSchemaWrapper>(rfc3339_offset).unwrap();
        assert_eq!(de.0.unix_timestamp(), 1033570800);

        let de = serde_json::from_str::<OffsetDateTimeJsonSchemaWrapper>("\"nonsense\"").unwrap();
        assert_eq!(de.0.unix_timestamp(), 0);
    }
}
