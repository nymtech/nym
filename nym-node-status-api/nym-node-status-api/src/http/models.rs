use std::net::IpAddr;

use crate::monitor::ExplorerPrettyBond;
use cosmwasm_std::{Addr, Coin, Decimal};
use nym_mixnet_contract_common::CoinSchema;
use nym_node_requests::api::v1::node::models::NodeDescription;
use nym_validator_client::{
    client::NodeId,
    models::{
        AuthenticatorDetails, BinaryBuildInformationOwned, IpPacketRouterDetails, NymNodeData,
    },
    nym_api::SkimmedNode,
    nym_nodes::{BasicEntryInformation, NodeRole},
};
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
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
pub struct BuildInformation {
    pub build_version: String,
    pub commit_branch: String,
    pub commit_sha: String,
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
    pub ip_packet_router: Option<IpPacketRouterDetails>,
    pub authenticator: Option<AuthenticatorDetails>,
    pub location: Location,
    pub last_probe: Option<DirectoryGwProbe>,
    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,
    pub mix_port: u16,
    pub role: NodeRole,
    pub entry: Option<BasicEntryInformation>,
    // The performance data here originates from the nym-api, and is effectively mixnet performance
    // at the time of writing this
    pub performance: String,
    pub build_information: BinaryBuildInformationOwned,
}

impl DVpnGateway {
    pub fn can_route_entry(&self) -> bool {
        self.last_probe
            .as_ref()
            .map(|probe| match &probe.outcome.as_entry {
                directory_gw_probe_outcome::Entry::Tested(entry_test_result) => {
                    entry_test_result.can_route
                }
                directory_gw_probe_outcome::Entry::NotTested
                | directory_gw_probe_outcome::Entry::EntryFailure => false,
            })
            .unwrap_or(false)
    }

    pub fn can_route_exit(&self) -> bool {
        self.last_probe
            .as_ref()
            .map(|probe| {
                probe
                    .outcome
                    .as_exit
                    .as_ref()
                    .map(|outcome| {
                        outcome.can_route_ip_external_v4 && outcome.can_route_ip_external_v6
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }
}

/// based on
/// https://github.com/nymtech/nym-vpn-client/blob/nym-vpn-core-v1.10.0/nym-vpn-core/crates/nym-gateway-probe/src/types.rs
/// TODO: long term types should be moved into this repo because nym-vpn-client
/// could pull it as a dependency and we'd have a single source of truth
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct LastProbeResult {
    node: String,
    used_entry: String,
    outcome: ProbeOutcome,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DirectoryGwProbe {
    last_updated_utc: String,
    outcome: ProbeOutcome,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ProbeOutcome {
    as_entry: directory_gw_probe_outcome::Entry,
    as_exit: Option<directory_gw_probe_outcome::Exit>,
    wg: Option<wg_outcome_versions::ProbeOutcomeV1>,
}

pub mod directory_gw_probe_outcome {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    #[serde(untagged)]
    #[allow(clippy::enum_variant_names)]
    pub enum Entry {
        Tested(EntryTestResult),
        NotTested,
        EntryFailure,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct EntryTestResult {
        pub can_connect: bool,
        pub can_route: bool,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub struct Exit {
        pub can_connect: bool,
        pub can_route_ip_v4: bool,
        pub can_route_ip_external_v4: bool,
        pub can_route_ip_v6: bool,
        pub can_route_ip_external_v6: bool,
    }
}

pub mod wg_outcome_versions {
    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub struct ProbeOutcomeV1 {
        pub can_register: bool,

        pub can_handshake_v4: bool,
        pub can_resolve_dns_v4: bool,
        pub ping_hosts_performance_v4: f32,
        pub ping_ips_performance_v4: f32,

        pub can_handshake_v6: bool,
        pub can_resolve_dns_v6: bool,
        pub ping_hosts_performance_v6: f32,
        pub ping_ips_performance_v6: f32,

        pub download_duration_sec_v4: u64,
        pub downloaded_file_v4: String,
        pub download_error_v4: String,

        pub download_duration_sec_v6: u64,
        pub downloaded_file_v6: String,
        pub download_error_v6: String,
    }
}

impl DVpnGateway {
    #[instrument(level = tracing::Level::INFO, name = "dvpn_gw_new", skip_all, fields(gateway_key = gateway.gateway_identity_key, node_id = skimmed_node.node_id))]
    pub(crate) fn new(gateway: Gateway, skimmed_node: &SkimmedNode) -> anyhow::Result<Self> {
        let location = gateway
            .explorer_pretty_bond
            .ok_or_else(|| anyhow::anyhow!("Missing explorer_pretty_bond"))
            .and_then(|value| {
                serde_json::from_value::<ExplorerPrettyBond>(value).map_err(From::from)
            })
            .map(|bond| bond.location)?;

        let self_described = gateway
            .self_described
            .ok_or_else(|| anyhow::anyhow!("Missing self_described"))
            .and_then(|value| serde_json::from_value::<NymNodeData>(value).map_err(From::from))?;

        let last_probe_result = match gateway.last_probe_result {
            Some(value) => {
                let parsed =
                    serde_json::from_value::<LastProbeResult>(value).inspect_err(|err| {
                        error!("Failed to deserialize probe result: {err}");
                    })?;
                Some(parsed)
            }
            None => None,
        };

        Ok(Self {
            identity_key: gateway.gateway_identity_key,
            name: gateway.description.moniker,
            ip_packet_router: self_described.ip_packet_router,
            authenticator: self_described.authenticator,
            location: Location {
                latitude: location.location.latitude,
                longitude: location.location.longitude,
                two_letter_iso_country_code: location.two_letter_iso_country_code,
            },
            last_probe: last_probe_result.map(|res| DirectoryGwProbe {
                last_updated_utc: gateway.last_testrun_utc.unwrap_or_default(),
                outcome: res.outcome,
            }),
            ip_addresses: skimmed_node.ip_addresses.clone(),
            mix_port: skimmed_node.mix_port,
            role: skimmed_node.role.clone(),
            entry: skimmed_node.entry.clone(),
            performance: to_percent(gateway.performance),
            build_information: self_described.build_information,
        })
    }
}

fn to_percent(performance: u8) -> String {
    let fraction = performance as f32 / 100.0;
    format!("{fraction:.2}")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn to_percent_should_work() {
        let starting = [0u8, 33, 50, 99, 100];
        let expected = ["0.00", "0.33", "0.50", "0.99", "1.00"];

        for (starting, expected) in starting.into_iter().zip(expected) {
            assert_eq!(expected, to_percent(starting));
        }
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
