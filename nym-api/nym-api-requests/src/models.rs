// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::unix_epoch;
use crate::legacy::{
    LegacyGatewayBondWithId, LegacyMixNodeBondWithLayer, LegacyMixNodeDetailsWithLayer,
};
use crate::nym_nodes::{BasicEntryInformation, NodeRole, SkimmedNode};
use crate::pagination::PaginatedResponse;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use nym_crypto::asymmetric::ed25519::{self, serde_helpers::bs58_ed25519_pubkey};
use nym_crypto::asymmetric::x25519::{
    self,
    serde_helpers::{bs58_x25519_pubkey, option_bs58_x25519_pubkey},
};
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::reward_params::{Performance, RewardingParams};
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::{GatewayBond, IdentityKey, Interval, MixNode, NodeId, Percent};
use nym_network_defaults::{DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT};
use nym_node_requests::api::v1::authenticator::models::Authenticator;
use nym_node_requests::api::v1::gateway::models::Wireguard;
use nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter;
use nym_node_requests::api::v1::node::models::{
    AuxiliaryDetails, BinaryBuildInformationOwned, NodeRoles,
};
use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::net::IpAddr;
use std::ops::{Deref, DerefMut};
use std::{fmt, time::Duration};
use thiserror::Error;
use time::{Date, OffsetDateTime};
use utoipa::{IntoParams, ToResponse, ToSchema};

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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
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
    pub count: i32,
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
    pub count: i32,
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
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
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
    pub last_24h_performance: Performance,
    pub current_role: Option<Role>,
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
    pub estimated_operator_apy: Decimal,
    pub estimated_delegators_apy: Decimal,
    pub blacklisted: bool,

    // a rather temporary thing until we query self-described endpoints of mixnodes
    #[serde(default)]
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
            role: role,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayBondAnnotated {
    pub gateway_bond: LegacyGatewayBondWithId,

    #[serde(default)]
    pub self_described: Option<GatewayDescription>,

    // NOTE: the performance field is deprecated in favour of node_performance
    pub performance: Performance,
    pub node_performance: NodePerformance,
    pub blacklisted: bool,

    #[serde(default)]
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
            role: role,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct GatewayDescription {
    // for now only expose what we need. this struct will evolve in the future (or be incorporated into nym-node properly)
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, ToSchema, IntoParams)]
pub struct ComputeRewardEstParam {
    #[schema(value_type = String)]
    pub performance: Option<Performance>,
    pub active_in_rewarded_set: Option<bool>,
    pub pledge_amount: Option<u64>,
    pub total_delegation: Option<u64>,
    #[schema(value_type = CoinSchema)]
    pub interval_operating_cost: Option<Coin>,
    #[schema(value_type = String)]
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

#[derive(Clone, Serialize, schemars::JsonSchema, ToSchema)]
pub struct AllInclusionProbabilitiesResponse {
    pub inclusion_probabilities: Vec<InclusionProbability>,
    pub samples: u64,
    pub elapsed: Duration,
    pub delta_max: f64,
    pub delta_l2: f64,
    pub as_at: i64,
}

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
    pub most_recent: Uptime,
    pub last_hour: Uptime,
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
    pub ed25519: ed25519::PublicKey,

    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    pub x25519: x25519::PublicKey,

    #[serde(default)]
    #[serde(with = "option_bs58_x25519_pubkey")]
    #[schemars(with = "Option<String>")]
    pub x25519_noise: Option<x25519::PublicKey>,
}

impl From<nym_node_requests::api::v1::node::models::HostKeys> for HostKeys {
    fn from(value: nym_node_requests::api::v1::node::models::HostKeys) -> Self {
        HostKeys {
            ed25519: value.ed25519_identity,
            x25519: value.x25519_sphinx,
            x25519_noise: value.x25519_noise,
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

    pub fn to_skimmed_node(&self, role: NodeRole, performance: Performance) -> SkimmedNode {
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
            x25519_sphinx_pubkey: keys.x25519,
            // we can't use the declared roles, we have to take whatever was provided in the contract.
            // why? say this node COULD operate as an exit, but it might be the case the contract decided
            // to assign it an ENTRY role only. we have to use that one instead.
            role: role,
            supported_roles: self.description.declared_role,
            entry,
            performance,
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
    pub uptime: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApiStatus {
    Up,
}

impl ApiHealthResponse {
    pub fn new_healthy(uptime: Duration) -> Self {
        ApiHealthResponse {
            status: ApiStatus::Up,
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

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct TestNode {
    pub node_id: Option<u32>,
    pub identity_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
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
pub struct NoiseDetails {
    #[schemars(with = "String")]
    #[serde(with = "bs58_x25519_pubkey")]
    pub x25119_pubkey: x25519::PublicKey,

    pub mixnet_port: u16,

    pub ip_addresses: Vec<IpAddr>,
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
