use cosmwasm_std::{Addr, Coin, Decimal};
use nym_mixnet_contract_common::CoinSchema;
use nym_node_requests::api::v1::node::models::NodeDescription;
use nym_validator_client::client::NodeId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub(crate) use nym_node_status_client::models::TestrunAssignment;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Gateway {
    pub gateway_identity_key: String,
    pub bonded: bool,
    pub performance: u8,
    pub self_described: Option<serde_json::Value>,
    pub explorer_pretty_bond: Option<serde_json::Value>,
    pub description: NodeDescription,
    pub last_probe_result: Option<serde_json::Value>,
    pub last_probe_log: Option<String>,
    pub last_testrun_utc: Option<String>,
    pub last_updated_utc: String,
    pub routing_score: f32,
    pub config_score: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub enum Role {
    // a properly active mixnode
    Mixnode {
        layer: u8,
    },

    #[serde(alias = "entry", alias = "gateway")]
    EntryGateway,

    #[serde(alias = "exit")]
    ExitGateway,

    // equivalent of node that's in rewarded set but not in the inactive set
    Standby,

    Inactive,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct BuildInformation {
    pub build_version: String,
    pub commit_branch: String,
    pub commit_sha: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct IpPacketRouter {
    pub address: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Authenticator {
    pub address: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EntryInformation {
    pub hostname: Option<String>,
    pub ws_port: u16,
    pub wss_port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Location {
    pub two_letter_iso_country_code: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DVpnGateway {
    pub identity_key: String,
    pub name: String,
    pub ip_packet_router: Option<IpPacketRouter>,
    pub authenticator: Option<Authenticator>,
    pub location: Location,
    pub last_probe: Option<serde_json::Value>,
    pub ip_addresses: Vec<String>,
    pub mix_port: u16,
    pub role: Role,
    pub entry: EntryInformation,
    // The performance data here originates from the nym-api, and is effectively mixnet performance
    // at the time of writing this
    pub performance: u8,
    pub build_information: Option<BuildInformation>,
}

impl TryFrom<Gateway> for DVpnGateway {
    type Error = ();

    fn try_from(value: Gateway) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&Gateway> for DVpnGateway
{
    type Error = ();

    fn try_from(value: &Gateway) -> Result<Self, Self::Error> {

        // TODO: try to parse out values from gateway and fail when unable to parse

        // TODO: polyfill `last_probe`, see below from VPN API
        /**

        const last_testrun_utc = item.last_testrun_utc;

        const last_probe: DirectoryGatewayProbe | undefined =
          last_testrun_utc && item.last_probe_result?.outcome
            ? {
                last_updated_utc: last_testrun_utc,
                outcome: item.last_probe_result.outcome,
              }
            : undefined;

        //
        // reshape test probe
        //
        if (
          last_probe?.outcome?.wg &&
          isDirectoryGatewayProbeOutcome_WG_V2(last_probe?.outcome?.wg) &&
          last_probe?.outcome?.wg?.can_handshake === undefined
        ) {
          last_probe.outcome.wg = {
            ...last_probe.outcome.wg,
            can_handshake: last_probe.outcome.wg.can_handshake_v4,
            can_resolve_dns: last_probe.outcome.wg.can_resolve_dns_v4,
            ping_hosts_performance:
              last_probe.outcome.wg.ping_hosts_performance_v4,
            ping_ips_performance: last_probe.outcome.wg.ping_ips_performance_v4,
          };
        }

         */

        Ok(Self {
            identity_key: value.gateway_identity_key.clone(),
            name: value.description.moniker.clone(),
            ip_packet_router: None, // value.ip_packet_router,
            authenticator: None, // value.authenticator,
            location: Location {
                latitude: 0.0f64,
                longitude: 0.0f64,
                two_letter_iso_country_code: "".to_string(),
            },
            last_probe: value.last_probe_result.clone(),
            ip_addresses: vec![], // value.ip_addresses,
            mix_port: 0u16, // value.mix_port,
            role: Role::EntryGateway, // value.role,
            entry: EntryInformation {
                ws_port: 0u16,
                wss_port: None,
                hostname: None,
            },
            performance: value.performance,
            build_information: None, // value.build_information,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct GatewaySkinny {
    pub gateway_identity_key: String,
    pub self_described: Option<serde_json::Value>,
    pub explorer_pretty_bond: Option<serde_json::Value>,
    pub last_probe_result: Option<serde_json::Value>,
    pub last_testrun_utc: Option<String>,
    pub last_updated_utc: String,
    pub routing_score: f32,
    pub config_score: u32,
    pub performance: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Mixnode {
    pub mix_id: u32,
    pub bonded: bool,
    pub is_dp_delegatee: bool,
    pub total_stake: i64,
    pub full_details: Option<serde_json::Value>,
    pub self_described: Option<serde_json::Value>,
    pub description: NodeDescription,
    pub last_updated_utc: String,
}

#[derive(Clone, Debug, utoipa::ToSchema, Deserialize, Serialize)]
pub(crate) struct ExtendedNymNode {
    pub(crate) node_id: NodeId,
    pub(crate) identity_key: String,
    pub(crate) uptime: f64,
    #[schema(value_type = String)]
    pub(crate) total_stake: Decimal,
    pub(crate) original_pledge: u128,
    pub(crate) bonding_address: Option<String>,
    pub(crate) bonded: bool,
    pub(crate) node_type: nym_validator_client::models::DescribedNodeType,
    pub(crate) ip_address: String,
    pub(crate) accepted_tnc: bool,
    pub(crate) self_description: nym_validator_client::models::NymNodeData,
    pub(crate) rewarding_details: Option<nym_mixnet_contract_common::NodeRewarding>,
    pub(crate) description: NodeDescription,
    pub(crate) geoip: Option<NodeGeoData>,
}

#[derive(Clone, Debug, utoipa::ToSchema, Deserialize, Serialize)]
pub(crate) struct NodeGeoData {
    pub(crate) city: String,
    pub(crate) country: String,
    pub(crate) ip_address: String,
    pub(crate) latitude: String,
    pub(crate) longitude: String,
    pub(crate) org: String,
    pub(crate) postal: String,
    pub(crate) region: String,
    pub(crate) timezone: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DailyStats {
    pub date_utc: String,
    pub total_packets_received: i64,
    pub total_packets_sent: i64,
    pub total_packets_dropped: i64,
    pub total_stake: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Service {
    pub gateway_identity_key: String,
    pub last_updated_utc: String,
    pub routing_score: f32,
    pub service_provider_client_id: Option<String>,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    pub mixnet_websockets: Option<serde_json::Value>,
    pub last_successful_ping_utc: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub(crate) struct SummaryHistory {
    pub date: String,
    pub value_json: serde_json::Value,
    pub timestamp_utc: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SessionStats {
    pub gateway_identity_key: String,
    pub node_id: u32,
    #[serde(with = "nym_serde_helpers::date")]
    pub day: time::Date,
    pub unique_active_clients: i64,
    pub session_started: i64,
    pub users_hashes: Option<serde_json::Value>,
    pub vpn_sessions: Option<serde_json::Value>,
    pub mixnet_sessions: Option<serde_json::Value>,
    pub unknown_sessions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct NodeDelegation {
    #[schema(value_type = CoinSchema)]
    pub amount: Coin,
    pub cumulative_reward_ratio: String,
    pub block_height: u64,
    #[schema(value_type = String)]
    pub owner: Addr,
    #[schema(value_type = Option<String>)]
    pub proxy: Option<Addr>,
}

impl From<nym_mixnet_contract_common::Delegation> for NodeDelegation {
    fn from(value: nym_mixnet_contract_common::Delegation) -> Self {
        Self {
            amount: value.amount,
            cumulative_reward_ratio: value.cumulative_reward_ratio.to_string(),
            block_height: value.height,
            owner: value.owner,
            proxy: value.proxy,
        }
    }
}
