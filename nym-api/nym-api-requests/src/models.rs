// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::pagination::PaginatedResponse;
use cosmwasm_std::{Addr, Coin, Decimal};
use nym_mixnet_contract_common::families::FamilyHead;
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::reward_params::{Performance, RewardingParams};
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::{
    GatewayBond, IdentityKey, Interval, MixId, MixNode, Percent, RewardedSetNodeStatus,
};
use nym_node_requests::api::v1::node::models::{AuxiliaryDetails, BinaryBuildInformationOwned};
use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};
use std::net::IpAddr;
use std::ops::{Deref, DerefMut};
use std::{fmt, time::Duration};
use time::OffsetDateTime;

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

    // a rather temporary thing until we query self-described endpoints of mixnodes
    #[serde(default)]
    pub ip_addresses: Vec<IpAddr>,
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

    #[serde(default)]
    pub ip_addresses: Vec<IpAddr>,
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

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HostKeys {
    pub ed25519: String,
    pub x25519: String,
}

impl From<nym_node_requests::api::v1::node::models::HostKeys> for HostKeys {
    fn from(value: nym_node_requests::api::v1::node::models::HostKeys) -> Self {
        HostKeys {
            ed25519: value.ed25519_identity,
            x25519: value.x25519_sphinx,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
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

const fn unix_epoch() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

pub fn de_rfc3339_or_default<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(time::serde::rfc3339::deserialize(deserializer).unwrap_or_else(|_| unix_epoch()))
}

pub(crate) mod overengineered_offset_date_time_serde {
    use crate::models::unix_epoch;
    use serde::de::Visitor;
    use serde::ser::Error;
    use serde::{Deserializer, Serialize, Serializer};
    use std::fmt::Formatter;
    use time::format_description::well_known::Rfc3339;
    use time::format_description::{modifier, BorrowedFormatItem, Component};
    use time::OffsetDateTime;

    struct OffsetDateTimeVisitor;

    // copied from time library because they keep it private -.-
    const DEFAULT_OFFSET_DATE_TIME_FORMAT: &[BorrowedFormatItem<'_>] = &[
        BorrowedFormatItem::Compound(DATE_FORMAT),
        BorrowedFormatItem::Literal(b" "),
        BorrowedFormatItem::Compound(TIME_FORMAT),
        BorrowedFormatItem::Literal(b" "),
        BorrowedFormatItem::Compound(UTC_OFFSET_FORMAT),
    ];

    const DATE_FORMAT: &[BorrowedFormatItem<'_>] = &[
        BorrowedFormatItem::Component(Component::Year(modifier::Year::default())),
        BorrowedFormatItem::Literal(b"-"),
        BorrowedFormatItem::Component(Component::Month(modifier::Month::default())),
        BorrowedFormatItem::Literal(b"-"),
        BorrowedFormatItem::Component(Component::Day(modifier::Day::default())),
    ];

    const TIME_FORMAT: &[BorrowedFormatItem<'_>] = &[
        BorrowedFormatItem::Component(Component::Hour(modifier::Hour::default())),
        BorrowedFormatItem::Literal(b":"),
        BorrowedFormatItem::Component(Component::Minute(modifier::Minute::default())),
        BorrowedFormatItem::Literal(b":"),
        BorrowedFormatItem::Component(Component::Second(modifier::Second::default())),
        BorrowedFormatItem::Literal(b"."),
        BorrowedFormatItem::Component(Component::Subsecond(modifier::Subsecond::default())),
    ];

    const UTC_OFFSET_FORMAT: &[BorrowedFormatItem<'_>] = &[
        BorrowedFormatItem::Component(Component::OffsetHour({
            let mut m = modifier::OffsetHour::default();
            m.sign_is_mandatory = true;
            m
        })),
        BorrowedFormatItem::Optional(&BorrowedFormatItem::Compound(&[
            BorrowedFormatItem::Literal(b":"),
            BorrowedFormatItem::Component(Component::OffsetMinute(
                modifier::OffsetMinute::default(),
            )),
            BorrowedFormatItem::Optional(&BorrowedFormatItem::Compound(&[
                BorrowedFormatItem::Literal(b":"),
                BorrowedFormatItem::Component(Component::OffsetSecond(
                    modifier::OffsetSecond::default(),
                )),
            ])),
        ])),
    ];

    impl<'de> Visitor<'de> for OffsetDateTimeVisitor {
        type Value = OffsetDateTime;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("an rfc3339 or human-readable `OffsetDateTime`")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // first try rfc3339, if that fails use default human-readable impl from time,
            // finally fallback to default unix epoch
            Ok(OffsetDateTime::parse(v, &Rfc3339).unwrap_or_else(|_| {
                OffsetDateTime::parse(v, &DEFAULT_OFFSET_DATE_TIME_FORMAT)
                    .unwrap_or_else(|_| unix_epoch())
            }))
        }
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(OffsetDateTimeVisitor)
    }

    pub(crate) fn serialize<S>(datetime: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize it with human-readable format for compatibility with eclipse and nutella clients
        // in the future change it back to rfc3339
        datetime
            .format(&DEFAULT_OFFSET_DATE_TIME_FORMAT)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }
}

// for all intents and purposes it's just OffsetDateTime, but we need JsonSchema...
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct OffsetDateTimeJsonSchemaWrapper(
    #[serde(default = "unix_epoch", with = "overengineered_offset_date_time_serde")]
    pub  OffsetDateTime,
);

impl Default for OffsetDateTimeJsonSchemaWrapper {
    fn default() -> Self {
        OffsetDateTimeJsonSchemaWrapper(unix_epoch())
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

// this struct is getting quite bloated...
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NymNodeDescription {
    #[serde(default)]
    pub last_polled: OffsetDateTimeJsonSchemaWrapper,

    pub host_information: HostInformation,

    #[serde(default)]
    pub auxiliary_details: AuxiliaryDetails,

    // TODO: do we really care about ALL build info or just the version?
    pub build_information: BinaryBuildInformationOwned,

    #[serde(default)]
    pub network_requester: Option<NetworkRequesterDetails>,

    #[serde(default)]
    pub ip_packet_router: Option<IpPacketRouterDetails>,

    // for now we only care about their ws/wss situation, nothing more
    pub mixnet_websockets: WebSockets,

    pub role: NymNodeRole,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NymNodeRole {
    Mixnode,
    Gateway,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ApiHealthResponse {
    pub status: ApiStatus,
    pub uptime: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PartialTestResult {
    pub monitor_run_id: i64,
    pub timestamp: i64,
    pub overall_reliability_for_all_routes_in_monitor_run: Option<u8>,
    pub test_routes: TestRoute,
}

pub type MixnodeTestResultResponse = PaginatedResponse<PartialTestResult>;
pub type GatewayTestResultResponse = PaginatedResponse<PartialTestResult>;

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
