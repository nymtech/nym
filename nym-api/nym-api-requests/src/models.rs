// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Coin, Decimal};
use nym_mixnet_contract_common::families::FamilyHead;
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::reward_params::{Performance, RewardingParams};
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::{
    GatewayBond, IdentityKey, Interval, MixId, MixNode, MixNodeBond, Percent, RewardedSetNodeStatus,
};
use nym_node_requests::api::v1::gateway::models::WebSockets;
use nym_node_requests::api::v1::node::models::{
    BinaryBuildInformationOwned, HostInformation, NoiseInformation,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::{fmt, time::Duration};

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
}

impl Display for RequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/MixnodeStatus.ts")
)]
#[serde(rename_all = "snake_case")]
pub enum MixnodeStatus {
    Active,   // in both the active set and the rewarded set
    Standby,  // only in the rewarded set
    Inactive, // in neither the rewarded set nor the active set, but is bonded
    NotFound, // doesn't even exist in the bonded set
}

impl From<MixnodeStatus> for Option<RewardedSetNodeStatus> {
    fn from(status: MixnodeStatus) -> Self {
        match status {
            MixnodeStatus::Active => Some(RewardedSetNodeStatus::Active),
            MixnodeStatus::Standby => Some(RewardedSetNodeStatus::Standby),
            MixnodeStatus::Inactive => None,
            MixnodeStatus::NotFound => None,
        }
    }
}

impl MixnodeStatus {
    pub fn is_active(&self) -> bool {
        *self == MixnodeStatus::Active
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/MixnodeCoreStatusResponse.ts")
)]
pub struct MixnodeCoreStatusResponse {
    pub mix_id: MixId,
    pub count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/GatewayCoreStatusResponse.ts")
)]
pub struct GatewayCoreStatusResponse {
    pub identity: String,
    pub count: i32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/MixnodeStatusResponse.ts")
)]
pub struct MixnodeStatusResponse {
    pub status: MixnodeStatus,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct NodePerformance {
    pub most_recent: Performance,
    pub last_hour: Performance,
    pub last_24h: Performance,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct MixNodeBondAnnotated {
    pub mixnode_details: MixNodeDetails,
    pub stake_saturation: StakeSaturation,
    pub uncapped_stake_saturation: StakeSaturation,
    // NOTE: the performance field is deprecated in favour of node_performance
    pub performance: Performance,
    pub node_performance: NodePerformance,
    pub estimated_operator_apy: Decimal,
    pub estimated_delegators_apy: Decimal,
    pub family: Option<FamilyHead>,
    pub blacklisted: bool,
}

impl MixNodeBondAnnotated {
    pub fn mix_node(&self) -> &MixNode {
        &self.mixnode_details.bond_information.mix_node
    }

    pub fn mix_id(&self) -> MixId {
        self.mixnode_details.mix_id()
    }

    pub fn identity_key(&self) -> &str {
        self.mixnode_details.bond_information.identity()
    }

    pub fn owner(&self) -> &Addr {
        self.mixnode_details.bond_information.owner()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct GatewayBondAnnotated {
    pub gateway_bond: GatewayBond,

    #[serde(default)]
    pub self_described: Option<GatewayDescription>,

    // NOTE: the performance field is deprecated in favour of node_performance
    pub performance: Performance,
    pub node_performance: NodePerformance,
    pub blacklisted: bool,
}

impl GatewayBondAnnotated {
    pub fn identity(&self) -> &String {
        self.gateway_bond.identity()
    }

    pub fn owner(&self) -> &Addr {
        self.gateway_bond.owner()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct GatewayDescription {
    // for now only expose what we need. this struct will evolve in the future (or be incorporated into nym-node properly)
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ComputeRewardEstParam {
    pub performance: Option<Performance>,
    pub active_in_rewarded_set: Option<bool>,
    pub pledge_amount: Option<u64>,
    pub total_delegation: Option<u64>,
    pub interval_operating_cost: Option<Coin>,
    pub profit_margin_percent: Option<Percent>,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/RewardEstimationResponse.ts")
)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RewardEstimationResponse {
    pub estimation: RewardEstimate,
    pub reward_params: RewardingParams,
    pub epoch: Interval,
    #[cfg_attr(feature = "generate-ts", ts(type = "number"))]
    pub as_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct UptimeResponse {
    pub mix_id: MixId,
    // The same as node_performance.last_24h. Legacy
    pub avg_uptime: u8,
    pub node_performance: NodePerformance,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct GatewayUptimeResponse {
    pub identity: String,
    // The same as node_performance.last_24h. Legacy
    pub avg_uptime: u8,
    pub node_performance: NodePerformance,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/StakeSaturationResponse.ts")
)]
pub struct StakeSaturationResponse {
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub saturation: StakeSaturation,

    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub uncapped_saturation: StakeSaturation,
    pub as_at: i64,
}

pub type StakeSaturation = Decimal;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/SelectionChance.ts")
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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/InclusionProbabilityResponse.ts")
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

#[derive(Clone, Serialize, schemars::JsonSchema)]
pub struct AllInclusionProbabilitiesResponse {
    pub inclusion_probabilities: Vec<InclusionProbability>,
    pub samples: u64,
    pub elapsed: Duration,
    pub delta_max: f64,
    pub delta_l2: f64,
    pub as_at: i64,
}

#[derive(Clone, Serialize, schemars::JsonSchema)]
pub struct InclusionProbability {
    pub mix_id: MixId,
    pub in_active: f64,
    pub in_reserve: f64,
}

type Uptime = u8;

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MixnodeStatusReportResponse {
    pub mix_id: MixId,
    pub identity: IdentityKey,
    pub owner: String,
    pub most_recent: Uptime,
    pub last_hour: Uptime,
    pub last_day: Uptime,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GatewayStatusReportResponse {
    pub identity: String,
    pub owner: String,
    pub most_recent: Uptime,
    pub last_hour: Uptime,
    pub last_day: Uptime,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HistoricalUptimeResponse {
    pub date: String,
    pub uptime: Uptime,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MixnodeUptimeHistoryResponse {
    pub mix_id: MixId,
    pub identity: String,
    pub owner: String,
    pub history: Vec<HistoricalUptimeResponse>,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GatewayUptimeHistoryResponse {
    pub identity: String,
    pub owner: String,
    pub history: Vec<HistoricalUptimeResponse>,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CirculatingSupplyResponse {
    pub total_supply: Coin,
    pub mixmining_reserve: Coin,
    pub vesting_tokens: Coin,
    pub circulating_supply: Coin,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NymNodeDescription {
    pub host_information: HostInformation,

    // TODO: do we really care about ALL build info or just the version?
    pub build_information: BinaryBuildInformationOwned,

    #[serde(default)]
    pub network_requester: Option<NetworkRequesterDetails>,

    #[serde(default)]
    pub ip_packet_router: Option<IpPacketRouterDetails>,

    // for now we only care about their ws/wss situation, nothing more
    #[serde(default)]
    pub mixnet_websockets: Option<WebSockets>,

    pub noise_information: NoiseInformation,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DescribedGateway {
    pub bond: GatewayBond,
    pub self_described: Option<NymNodeDescription>,
}

impl From<GatewayBond> for DescribedGateway {
    fn from(bond: GatewayBond) -> Self {
        DescribedGateway {
            bond,
            self_described: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DescribedMixnode {
    pub bond: MixNodeBond,
    pub self_described: Option<NymNodeDescription>,
}

impl From<MixNodeBond> for DescribedMixnode {
    fn from(bond: MixNodeBond) -> Self {
        DescribedMixnode {
            bond,
            self_described: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub enum DescribedNymNode {
    Gateway(DescribedGateway),
    Mixnode(DescribedMixnode),
}

impl From<GatewayBond> for DescribedNymNode {
    fn from(value: GatewayBond) -> Self {
        Self::Gateway(value.into())
    }
}

impl From<MixNodeDetails> for DescribedNymNode {
    fn from(value: MixNodeDetails) -> Self {
        Self::Mixnode(value.bond_information.into())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NetworkRequesterDetails {
    /// address of the embedded network requester
    pub address: String,

    /// flag indicating whether this network requester uses the exit policy rather than the deprecated allow list
    pub uses_exit_policy: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct IpPacketRouterDetails {
    /// address of the embedded ip packet router
    pub address: String,
}
