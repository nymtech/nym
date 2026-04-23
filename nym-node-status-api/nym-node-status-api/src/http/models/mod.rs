use std::collections::HashMap;
use std::net::IpAddr;

use crate::{
    http::models::gw_probe::{
        DvpnGwProbe, DvpnProbeOutcome, LastProbeResult, ScoreValue, calc_gateway_visual_score,
        calculate_load,
    },
    monitor::ExplorerPrettyBond,
};
use cosmwasm_std::{Addr, Coin, Decimal};
use nym_mixnet_contract_common::CoinSchema;
use nym_node_requests::api::v1::node::models::NodeDescription;
use nym_validator_client::{
    client::NodeId,
    models::{
        AuthenticatorDetailsV1, BinaryBuildInformationOwned, IpPacketRouterDetailsV1,
        LewesProtocolDetailsDataV1 as LewesProtocolDetailsDataV1Validator,
        LewesProtocolDetailsV1 as LewesProtocolDetailsV1Validator,
    },
    nym_api::SkimmedNodeV1,
    nym_nodes::{BasicEntryInformation, NodeRole},
};
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use tracing::{error, instrument};
use utoipa::ToSchema;

use crate::db::models::NymNodeDataDeHelper;
use crate::node_scraper::models::BridgeInformation;

pub(crate) use nym_node_status_client::models::TestrunAssignment;

pub(crate) mod gw_probe;

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
    pub ports_check: Option<serde_json::Value>,
    pub last_ports_check_utc: Option<String>,
    pub last_testrun_utc: Option<String>,
    pub last_updated_utc: String,
    pub routing_score: f32,
    pub config_score: u32,
    pub bridges: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct BuildInformation {
    pub build_version: String,
    pub commit_branch: String,
    pub commit_sha: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AsnKind {
    Residential,
    Other,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Asn {
    pub asn: String,
    pub name: String,
    pub domain: String,
    pub route: String,
    pub kind: AsnKind,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Location {
    pub two_letter_iso_country_code: String,
    pub latitude: f64,
    pub longitude: f64,

    pub city: String,
    pub region: String,
    pub org: String,
    pub postal: String,
    pub timezone: String,

    pub asn: Option<Asn>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DVpnGatewayPerformance {
    last_updated_utc: String,
    score: ScoreValue,
    mixnet_score: ScoreValue,
    load: ScoreValue,
    uptime_percentage_last_24_hours: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct LewesProtocolDetailsV1 {
    pub content: LewesProtocolDetailsDataV1,
    pub signature: String,
}

impl From<&LewesProtocolDetailsV1Validator> for LewesProtocolDetailsV1 {
    fn from(value: &LewesProtocolDetailsV1Validator) -> Self {
        Self {
            content: (&value.content).into(),
            signature: value.signature.to_base58_string(),
        }
    }
}

// maps from a type in nym validator client: copied over doc comments for a prettier OpenAPI spec :)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct LewesProtocolDetailsDataV1 {
    /// Helper field that specifies whether the LP listener(s) is enabled on this node.
    /// It is directly controlled by the node's role (i.e. it is enabled if it supports 'entry' mode)
    pub enabled: bool,
    /// LP TCP control address (default: 41264) for establishing LP sessions
    pub control_port: u16,
    /// LP UDP data address (default: 51264) for Sphinx packets wrapped in LP
    pub data_port: u16,
    /// LP public key
    pub x25519: String,
    /// Digests of the KEM keys available to this node alongside hashing algorithms used
    /// for their computation.
    /// note: digests are hex encoded
    pub kem_keys: HashMap<String, HashMap<String, String>>,
}

impl From<&LewesProtocolDetailsDataV1Validator> for LewesProtocolDetailsDataV1 {
    fn from(value: &LewesProtocolDetailsDataV1Validator) -> Self {
        let x25519_pk: nym_crypto::asymmetric::x25519::PublicKey = value.x25519.into();

        LewesProtocolDetailsDataV1 {
            enabled: value.enabled,
            control_port: value.control_port,
            data_port: value.data_port,
            x25519: x25519_pk.to_base58_string(),
            kem_keys: value
                .kem_keys
                .iter()
                .map(|(kem, digests)| {
                    (
                        kem.to_string(),
                        digests
                            .iter()
                            .map(|(hash_fn, digest)| (hash_fn.to_string(), digest.clone()))
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DVpnGateway {
    pub identity_key: String,
    pub name: String,
    pub description: Option<String>,
    pub ip_packet_router: Option<IpPacketRouterDetailsV1>,
    pub authenticator: Option<AuthenticatorDetailsV1>,
    pub location: Location,
    pub last_probe: Option<DvpnGwProbe>,
    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,
    pub mix_port: u16,
    pub role: NodeRole,
    pub entry: Option<BasicEntryInformation>,
    pub bridges: Option<BridgeInformation>,

    // The performance data here originates from the nym-api, and is effectively mixnet performance
    // at the time of writing this
    pub performance: String,

    // Node performance information needed by the NymVPN UI / Explorer to show more information
    // about the node in a user-friendly way
    pub performance_v2: Option<DVpnGatewayPerformance>,

    pub lewes_protocol_details: Option<LewesProtocolDetailsV1>,

    pub build_information: BinaryBuildInformationOwned,
}

impl DVpnGateway {
    pub fn can_route_entry(&self) -> bool {
        self.last_probe
            .as_ref()
            .map(DvpnGwProbe::can_route_entry)
            .unwrap_or(false)
    }

    pub fn can_route_exit(&self) -> bool {
        self.last_probe
            .as_ref()
            .map(|probe| probe.can_route_exit().unwrap_or(false))
            .unwrap_or(false)
    }
}

impl DVpnGateway {
    #[instrument(level = tracing::Level::INFO, name = "dvpn_gw_new", skip_all, fields(gateway_key = gateway.gateway_identity_key, node_id = skimmed_node.node_id))]
    pub(crate) fn new(
        gateway: Gateway,
        skimmed_node: &SkimmedNodeV1,
        socks5_score: Option<&ScoreValue>,
    ) -> anyhow::Result<Self> {
        let location = gateway
            .explorer_pretty_bond
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing explorer_pretty_bond"))
            .and_then(|value| {
                serde_json::from_value::<ExplorerPrettyBond>(value).map_err(From::from)
            })
            .map(|bond| bond.location)?;

        let self_described: NymNodeDataDeHelper = gateway
            .self_described
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing self_described"))
            .and_then(|value| {
                serde_json::from_value::<NymNodeDataDeHelper>(value).map_err(From::from)
            })?;

        let last_updated_utc = gateway.last_testrun_utc.clone().unwrap_or_default();
        let performance = to_percent(gateway.performance);
        let network_monitor_performance_mixnet_mode = gateway.performance as f32 / 100f32;
        let bridges = gateway.bridges.clone().and_then(|v| {
            serde_json::from_value(v)
                .inspect_err(|err| {
                    error!(
                        "Failed to deserialize bridges for gateway identity {}: {err}",
                        gateway.gateway_identity_key
                    );
                })
                .ok()
        });

        tracing::debug!("🌈 gateway probe result: {:?}", gateway.last_probe_result);

        let (last_probe_result, performance_v2) = match gateway.last_probe_result {
            Some(ref value) => {
                let parsed = LastProbeResult::deserialize_with_fallback(value.clone())
                    .inspect_err(|err| {
                        error!("Failed to deserialize probe result: {err}");
                    })?;

                tracing::trace!("🌈 gateway probe parsed: {:?}", parsed);
                let mixnet_score = calculate_mixnet_score(&gateway);
                let score = calc_gateway_visual_score(&gateway, &parsed);
                let mut load = calculate_load(&parsed);
                let socks5_score = socks5_score.unwrap_or(&ScoreValue::Offline).to_owned();
                let dvpn_probe_result =
                    DvpnProbeOutcome::from_raw_probe_outcome(parsed.outcome(), socks5_score);

                // clamp the load value to offline, when the score is offline
                if score == ScoreValue::Offline {
                    load = ScoreValue::Offline;
                }

                let performance_v2 = DVpnGatewayPerformance {
                    last_updated_utc: last_updated_utc.to_string(),
                    load,
                    score,
                    mixnet_score,

                    // the network monitor's measure is a good proxy for node uptime, it can be improved in the future
                    uptime_percentage_last_24_hours: network_monitor_performance_mixnet_mode,
                };
                (Some(dvpn_probe_result), Some(performance_v2))
            }
            None => (None, None),
        };

        Ok(Self {
            identity_key: gateway.gateway_identity_key,
            name: gateway.description.moniker,
            description: Some(gateway.description.details),
            ip_packet_router: self_described.ip_packet_router,
            authenticator: self_described.authenticator,
            location: Location {
                latitude: location.location.latitude,
                longitude: location.location.longitude,
                two_letter_iso_country_code: location.two_letter_iso_country_code,
                org: location.org,
                city: location.city,
                region: location.region,
                postal: location.postal,
                timezone: location.timezone,
                asn: location.asn.map(|a| {
                    let kind = if a.kind.eq_ignore_ascii_case("isp") {
                        // we consider anything that is "ISP" from ipinfo to be residential
                        AsnKind::Residential
                    } else {
                        // everything else is considered "other"
                        AsnKind::Other
                    };
                    Asn {
                        asn: a.asn,
                        domain: a.domain,
                        kind,
                        name: a.name,
                        route: a.route,
                    }
                }),
            },
            last_probe: last_probe_result
                .map(|res| DvpnGwProbe::from_outcome(res, last_updated_utc)),
            ip_addresses: skimmed_node.ip_addresses.clone(),
            mix_port: skimmed_node.mix_port,
            role: skimmed_node.role.clone(),
            entry: skimmed_node.entry.clone(),
            bridges,
            performance,
            performance_v2,
            lewes_protocol_details: self_described
                .lewes_protocol
                .as_ref()
                .map(LewesProtocolDetailsV1::from),
            build_information: self_described.build_information,
        })
    }
}

/// calculates the gateway probe score for mixnet mode
fn calculate_mixnet_score(gateway: &Gateway) -> ScoreValue {
    let mixnet_performance = gateway.performance as f64 / 100.0;

    if mixnet_performance > 0.8 {
        ScoreValue::High
    } else if mixnet_performance > 0.6 {
        ScoreValue::Medium
    } else if mixnet_performance > 0.1 {
        ScoreValue::Low
    } else {
        ScoreValue::Offline
    }
}

fn to_percent(performance: u8) -> String {
    let fraction = performance as f32 / 100.0;
    format!("{fraction:.2}")
}

#[cfg(test)]
mod test {

    use super::*;
    use std::str::FromStr;

    #[test]
    fn to_percent_should_work() {
        let starting = [0u8, 33, 50, 99, 100];
        let expected = ["0.00", "0.33", "0.50", "0.99", "1.00"];

        for (starting, expected) in starting.into_iter().zip(expected) {
            assert_eq!(expected, to_percent(starting));
        }
    }

    #[test]
    fn to_percent_edge_cases() {
        // Test edge cases
        assert_eq!("0.00", to_percent(0));
        assert_eq!("1.00", to_percent(100));
        assert_eq!("2.55", to_percent(255)); // Over 100%
    }

    #[test]
    fn node_delegation_from_conversion() {
        use cosmwasm_std::Uint128;

        let delegation = nym_mixnet_contract_common::Delegation {
            node_id: 42,
            amount: Coin {
                denom: "unym".to_string(),
                amount: Uint128::new(1000000),
            },
            cumulative_reward_ratio: Decimal::from_str("1.23456789").unwrap(),
            height: 12345,
            owner: Addr::unchecked("owner1"),
            proxy: Some(Addr::unchecked("proxy1")),
        };

        let node_delegation: NodeDelegation = delegation.clone().into();

        assert_eq!(node_delegation.amount.denom, "unym");
        assert_eq!(node_delegation.amount.amount, Uint128::new(1000000));
        assert_eq!(node_delegation.cumulative_reward_ratio, "1.23456789");
        assert_eq!(node_delegation.block_height, 12345);
        assert_eq!(node_delegation.owner, Addr::unchecked("owner1"));
        assert_eq!(node_delegation.proxy, Some(Addr::unchecked("proxy1")));
    }

    #[test]
    fn node_delegation_from_conversion_no_proxy() {
        use cosmwasm_std::Uint128;

        let delegation = nym_mixnet_contract_common::Delegation {
            node_id: 0,
            amount: Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(0),
            },
            cumulative_reward_ratio: Decimal::zero(),
            height: 0,
            owner: Addr::unchecked("owner2"),
            proxy: None,
        };

        let node_delegation: NodeDelegation = delegation.into();

        assert_eq!(node_delegation.amount.denom, "uatom");
        assert_eq!(node_delegation.amount.amount, Uint128::new(0));
        assert_eq!(node_delegation.cumulative_reward_ratio, "0");
        assert_eq!(node_delegation.block_height, 0);
        assert_eq!(node_delegation.owner, Addr::unchecked("owner2"));
        assert_eq!(node_delegation.proxy, None);
    }

    #[test]
    fn node_delegation_from_conversion_max_values() {
        use cosmwasm_std::Uint128;

        let delegation = nym_mixnet_contract_common::Delegation {
            node_id: u32::MAX,
            amount: Coin {
                denom: "test".to_string(),
                amount: Uint128::MAX,
            },
            cumulative_reward_ratio: Decimal::from_str("999999999.999999999").unwrap(),
            height: u64::MAX,
            owner: Addr::unchecked("owner3"),
            proxy: Some(Addr::unchecked("proxy3")),
        };

        let node_delegation: NodeDelegation = delegation.into();

        assert_eq!(node_delegation.amount.amount, Uint128::MAX);
        assert_eq!(
            node_delegation.cumulative_reward_ratio,
            "999999999.999999999"
        );
        assert_eq!(node_delegation.block_height, u64::MAX);
    }

    #[test]
    fn location_struct_creation() {
        let location = Location {
            two_letter_iso_country_code: "US".to_string(),
            latitude: 40.7128,
            longitude: -74.0060,
            org: "Nym".to_string(),
            city: "Genève".to_string(),
            region: "Geneva".to_string(),
            postal: "1200".to_string(),
            timezone: "Europe/Zurich".to_string(),
            asn: None,
        };

        assert_eq!(location.two_letter_iso_country_code, "US");
        assert_eq!(location.latitude, 40.7128);
        assert_eq!(location.longitude, -74.0060);
    }

    #[test]
    fn location_extreme_coordinates() {
        // Test extreme coordinates
        let north_pole = Location {
            two_letter_iso_country_code: "XX".to_string(),
            latitude: 90.0,
            longitude: 0.0,
            org: "Nym".to_string(),
            city: "Genève".to_string(),
            region: "Geneva".to_string(),
            postal: "1200".to_string(),
            timezone: "Europe/Zurich".to_string(),
            asn: None,
        };

        let south_pole = Location {
            two_letter_iso_country_code: "AQ".to_string(),
            latitude: -90.0,
            longitude: 0.0,
            org: "Nym".to_string(),
            city: "Genève".to_string(),
            region: "Geneva".to_string(),
            postal: "1200".to_string(),
            timezone: "Europe/Zurich".to_string(),
            asn: None,
        };

        let date_line = Location {
            two_letter_iso_country_code: "FJ".to_string(),
            latitude: -17.0,
            longitude: 180.0,
            org: "Nym".to_string(),
            city: "Genève".to_string(),
            region: "Geneva".to_string(),
            postal: "1200".to_string(),
            timezone: "Europe/Zurich".to_string(),
            asn: None,
        };

        assert_eq!(north_pole.latitude, 90.0);
        assert_eq!(south_pole.latitude, -90.0);
        assert_eq!(date_line.longitude, 180.0);
    }

    #[test]
    fn build_information_creation() {
        let build_info = BuildInformation {
            build_version: "1.2.3".to_string(),
            commit_branch: "main".to_string(),
            commit_sha: "abcdef123456".to_string(),
        };

        assert_eq!(build_info.build_version, "1.2.3");
        assert_eq!(build_info.commit_branch, "main");
        assert_eq!(build_info.commit_sha, "abcdef123456");
    }

    #[test]
    fn daily_stats_creation() {
        let stats = DailyStats {
            date_utc: "2024-01-20".to_string(),
            total_packets_received: 1_000_000,
            total_packets_sent: 999_000,
            total_packets_dropped: 1_000,
            total_stake: 5_000_000,
        };

        assert_eq!(stats.date_utc, "2024-01-20");
        assert_eq!(stats.total_packets_received, 1_000_000);
        assert_eq!(stats.total_packets_sent, 999_000);
        assert_eq!(stats.total_packets_dropped, 1_000);
        assert_eq!(stats.total_stake, 5_000_000);
    }

    #[test]
    fn daily_stats_negative_values() {
        // Test with edge case values
        let stats = DailyStats {
            date_utc: "".to_string(),
            total_packets_received: i64::MAX,
            total_packets_sent: 0,
            total_packets_dropped: -1, // Should this be allowed?
            total_stake: i64::MIN,
        };

        assert_eq!(stats.date_utc, "");
        assert_eq!(stats.total_packets_received, i64::MAX);
        assert_eq!(stats.total_packets_sent, 0);
        assert_eq!(stats.total_packets_dropped, -1);
        assert_eq!(stats.total_stake, i64::MIN);
    }

    #[test]
    fn gateway_skinny_creation() {
        let gateway = GatewaySkinny {
            gateway_identity_key: "gateway123".to_string(),
            self_described: Some(serde_json::json!({"test": "value"})),
            explorer_pretty_bond: None,
            last_probe_result: Some(serde_json::json!({"status": "ok"})),
            ports_check: None,
            last_ports_check_utc: None,
            last_testrun_utc: Some("2024-01-20T10:00:00Z".to_string()),
            last_updated_utc: "2024-01-20T11:00:00Z".to_string(),
            routing_score: 0.95,
            config_score: 100,
            performance: 98,
        };

        assert_eq!(gateway.gateway_identity_key, "gateway123");
        assert!(gateway.self_described.is_some());
        assert!(gateway.explorer_pretty_bond.is_none());
        assert_eq!(gateway.performance, 98);
        assert_eq!(gateway.routing_score, 0.95);
    }

    #[test]
    fn service_creation_with_all_fields() {
        let service = Service {
            gateway_identity_key: "gw123".to_string(),
            last_updated_utc: "2024-01-20T10:00:00Z".to_string(),
            routing_score: 0.85,
            service_provider_client_id: Some("client123".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            hostname: Some("gateway.example.com".to_string()),
            mixnet_websockets: Some(serde_json::json!({"port": 8080})),
            last_successful_ping_utc: Some("2024-01-20T09:55:00Z".to_string()),
        };

        assert_eq!(service.gateway_identity_key, "gw123");
        assert_eq!(service.routing_score, 0.85);
        assert_eq!(service.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(service.hostname, Some("gateway.example.com".to_string()));
    }

    #[test]
    fn service_creation_minimal() {
        let service = Service {
            gateway_identity_key: "gw456".to_string(),
            last_updated_utc: "2024-01-20T10:00:00Z".to_string(),
            routing_score: 0.0,
            service_provider_client_id: None,
            ip_address: None,
            hostname: None,
            mixnet_websockets: None,
            last_successful_ping_utc: None,
        };

        assert_eq!(service.gateway_identity_key, "gw456");
        assert_eq!(service.routing_score, 0.0);
        assert!(service.service_provider_client_id.is_none());
        assert!(service.ip_address.is_none());
        assert!(service.hostname.is_none());
        assert!(service.mixnet_websockets.is_none());
        assert!(service.last_successful_ping_utc.is_none());
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct GatewaySkinny {
    pub gateway_identity_key: String,
    pub self_described: Option<serde_json::Value>,
    pub explorer_pretty_bond: Option<serde_json::Value>,
    pub last_probe_result: Option<serde_json::Value>,
    pub ports_check: Option<serde_json::Value>,
    pub last_ports_check_utc: Option<String>,
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
    pub(crate) node_type: nym_validator_client::models::DescribedNodeTypeV1,
    pub(crate) ip_address: String,
    pub(crate) accepted_tnc: bool,
    pub(crate) self_description: nym_validator_client::models::NymNodeDataV2,
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
