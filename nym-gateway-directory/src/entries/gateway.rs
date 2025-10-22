// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use itertools::Itertools;
use nym_contracts_common::Percent;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_topology::{NodeId, RoutingNode};
use nym_validator_client::models::{KeyRotationId, NymNodeDescription};
use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};
use tracing::error;

use crate::{
    AuthAddress, BridgeInformation, BridgeParameters, Country, Error, IpPacketRouterAddress,
    ScoreThresholds,
    entries::score::{HIGH_SCORE_THRESHOLD, LOW_SCORE_THRESHOLD, MEDIUM_SCORE_THRESHOLD, Score},
    error::Result,
    helpers,
};

pub const COUNTRY_WITH_REGION_SELECTOR: &str = "US";

#[derive(Clone)]
pub struct Gateway {
    pub identity: NodeIdentity,
    pub moniker: String,
    pub location: Option<Location>,
    pub ipr_address: Option<IpPacketRouterAddress>,
    pub authenticator_address: Option<AuthAddress>,
    pub bridge_params: Option<BridgeInformation>,
    pub last_probe: Option<Probe>,
    pub ips: Vec<IpAddr>,
    pub host: Option<String>,
    pub clients_ws_port: Option<u16>,
    pub clients_wss_port: Option<u16>,
    pub mixnet_performance: Option<Percent>,
    pub mixnet_score: Option<Score>,
    pub wg_performance: Option<Performance>,
    pub version: Option<String>,
}

impl fmt::Debug for Gateway {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Gateway")
            .field("identity", &self.identity.to_base58_string())
            .field("location", &self.location)
            .field("ipr_address", &self.ipr_address)
            .field("authenticator_address", &self.authenticator_address)
            .field("last_probe", &self.last_probe)
            .field("host", &self.host)
            .field("clients_ws_port", &self.clients_ws_port)
            .field("clients_wss_port", &self.clients_wss_port)
            .field("mixnet_performance", &self.mixnet_performance)
            .field("mixnet_score", &self.mixnet_score)
            .field("wg_performance", &self.wg_performance)
            .field("version", &self.version)
            .finish()
    }
}

impl Gateway {
    pub fn try_from_node_description(
        node_description: NymNodeDescription,
        current_key_rotation: KeyRotationId,
    ) -> Result<Self> {
        let identity = node_description.description.host_information.keys.ed25519;
        let location = node_description
            .description
            .auxiliary_details
            .location
            .map(|l| Location {
                two_letter_iso_country_code: l.alpha2.to_string(),
                ..Default::default()
            });
        let ipr_address = node_description
            .description
            .ip_packet_router
            .as_ref()
            .and_then(|ipr| {
                IpPacketRouterAddress::try_from_base58_string(&ipr.address)
                    .inspect_err(|err| error!("Failed to parse IPR address: {err}"))
                    .ok()
            });
        let authenticator_address = node_description
            .description
            .authenticator
            .as_ref()
            .and_then(|a| {
                AuthAddress::try_from_base58_string(&a.address)
                    .inspect_err(|err| error!("Failed to parse authenticator address: {err}"))
                    .ok()
            });
        let version = Some(node_description.version().to_string());
        let role = if node_description.description.declared_role.entry {
            nym_validator_client::nym_nodes::NodeRole::EntryGateway
        } else if node_description.description.declared_role.exit_ipr
            || node_description.description.declared_role.exit_nr
        {
            nym_validator_client::nym_nodes::NodeRole::ExitGateway
        } else {
            nym_validator_client::nym_nodes::NodeRole::Inactive
        };

        let gateway = RoutingNode::try_from(&node_description.to_skimmed_node(
            current_key_rotation,
            role,
            Default::default(),
        ))
        .map_err(|_| Error::MalformedGateway)?;

        let host = gateway.ws_entry_address(false);
        let entry_info = &gateway.entry;
        let clients_ws_port = entry_info.as_ref().map(|g| g.clients_ws_port);
        let clients_wss_port = entry_info.as_ref().and_then(|g| g.clients_wss_port);
        let ips = node_description.description.host_information.ip_address;
        Ok(Gateway {
            identity,
            moniker: String::new(),
            location,
            ipr_address,
            authenticator_address,
            bridge_params: None,
            last_probe: None,
            ips,
            host,
            clients_ws_port,
            clients_wss_port,
            mixnet_performance: None,
            mixnet_score: None,
            wg_performance: None,
            version,
        })
    }

    pub fn identity(&self) -> NodeIdentity {
        self.identity
    }

    pub fn two_letter_iso_country_code(&self) -> Option<&str> {
        self.location
            .as_ref()
            .map(|l| l.two_letter_iso_country_code.as_str())
    }

    pub fn is_in_country(&self, two_letter_iso_country_code: &str) -> bool {
        self.location
            .as_ref()
            .map(|loc| loc.two_letter_iso_country_code == two_letter_iso_country_code)
            .unwrap_or(false)
    }

    pub fn region(&self) -> Option<&str> {
        self.location.as_ref().map(|l| l.region.as_str())
    }

    pub fn is_in_region(&self, region: &str) -> bool {
        self.location
            .as_ref()
            .map(|loc| loc.region == region)
            .unwrap_or(false)
    }

    pub fn is_residential_asn(&self) -> bool {
        self.location
            .as_ref()
            .and_then(|loc| loc.asn.as_ref())
            .map(|asn| asn.kind == AsnKind::Residential)
            .unwrap_or(false)
    }

    pub fn is_exit_node(&self) -> bool {
        self.ipr_address.is_some()
    }

    pub fn is_vpn_node(&self) -> bool {
        self.authenticator_address.is_some()
    }

    pub fn host(&self) -> Option<&String> {
        self.host.as_ref()
    }

    pub fn lookup_ip(&self) -> Option<IpAddr> {
        self.ips.first().copied()
    }

    pub fn split_ips(&self) -> (Vec<Ipv4Addr>, Vec<Ipv6Addr>) {
        helpers::split_ips(self.ips.clone())
    }

    pub fn clients_address_no_tls(&self) -> Option<String> {
        match (&self.host, &self.clients_ws_port) {
            (Some(host), Some(port)) => Some(format!("ws://{host}:{port}")),
            _ => None,
        }
    }

    pub fn clients_address_tls(&self) -> Option<String> {
        match (&self.host, &self.clients_wss_port) {
            (Some(host), Some(port)) => Some(format!("wss://{host}:{port}")),
            _ => None,
        }
    }

    pub fn update_to_new_thresholds(&mut self, mix_thresholds: Option<ScoreThresholds>) {
        if let (Some(mix_thresholds), Some(score)) = (mix_thresholds, self.mixnet_score.as_mut()) {
            score.update_to_new_thresholds(mix_thresholds);
        }
    }

    pub fn meets_score(&self, gw_type: Option<GatewayType>, min_score: ScoreValue) -> bool {
        match gw_type {
            Some(GatewayType::MixnetEntry) | Some(GatewayType::MixnetExit) => self
                .mixnet_performance
                .is_some_and(|p| p.round_to_integer() >= min_score.threshold()),
            Some(GatewayType::Wg) => self
                .wg_performance
                .as_ref()
                .is_some_and(|p| p.score >= min_score),
            None => false,
        }
    }

    /// Tests whether the gateway matches a specific filter.
    pub fn matches_filter(&self, gw_type: Option<GatewayType>, filter: &GatewayFilter) -> bool {
        match filter {
            GatewayFilter::MinScore(score) => self.meets_score(gw_type, *score),
            GatewayFilter::Country(code) => self.is_in_country(code),
            GatewayFilter::Region(region) => self.is_in_region(region),
            GatewayFilter::Residential => self.is_residential_asn(),
            GatewayFilter::Exit => self.is_exit_node(),
            GatewayFilter::Vpn => self.is_vpn_node(),
        }
    }

    /// Tests whether the gateway matches all of the filters.
    pub fn matches_all_filters(
        &self,
        gw_type: Option<GatewayType>,
        filters: &[GatewayFilter],
    ) -> bool {
        filters
            .iter()
            .all(|filter| self.matches_filter(gw_type, filter))
    }

    pub fn get_bridge_params(&self) -> Option<BridgeParameters> {
        if let Some(all_params) = &self.bridge_params {
            all_params.transports.first().cloned()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AsnKind {
    Residential,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Asn {
    pub asn: String,
    pub name: String,
    pub kind: AsnKind,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Location {
    pub two_letter_iso_country_code: String,
    pub latitude: f64,
    pub longitude: f64,

    pub city: String,
    pub region: String,

    pub asn: Option<Asn>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreValue {
    Offline,
    Low,
    Medium,
    High,
}

impl ScoreValue {
    fn priority(&self) -> u8 {
        match self {
            ScoreValue::Offline => 0,
            ScoreValue::Low => 1,
            ScoreValue::Medium => 2,
            ScoreValue::High => 3,
        }
    }

    pub fn threshold(&self) -> u8 {
        match self {
            ScoreValue::Offline => 0,
            ScoreValue::Low => LOW_SCORE_THRESHOLD,
            ScoreValue::Medium => MEDIUM_SCORE_THRESHOLD,
            ScoreValue::High => HIGH_SCORE_THRESHOLD,
        }
    }
}

impl PartialOrd for ScoreValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.priority().cmp(&other.priority()))
    }
}

impl FromStr for ScoreValue {
    type Err = crate::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "offline" => Ok(ScoreValue::Offline),
            "low" => Ok(ScoreValue::Low),
            "medium" => Ok(ScoreValue::Medium),
            "high" => Ok(ScoreValue::High),
            _ => Err(crate::Error::InvalidScoreValue(s.to_string())),
        }
    }
}

impl Display for ScoreValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ScoreValue::Offline => "Offline",
            ScoreValue::Low => "Low",
            ScoreValue::Medium => "Medium",
            ScoreValue::High => "High",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Performance {
    pub last_updated_utc: String,
    pub score: ScoreValue,
    pub load: ScoreValue,
    pub uptime_percentage_last_24_hours: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Probe {
    pub last_updated_utc: String,
    pub outcome: ProbeOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProbeOutcome {
    pub as_entry: Entry,
    pub as_exit: Option<Exit>,
    pub wg: Option<WgProbeResults>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub can_connect: bool,
    pub can_route: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exit {
    pub can_connect: bool,
    pub can_route_ip_v4: bool,
    pub can_route_ip_external_v4: bool,
    pub can_route_ip_v6: bool,
    pub can_route_ip_external_v6: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WgProbeResults {
    pub can_register: bool,
    pub can_handshake: bool,
    pub can_resolve_dns: bool,
    pub ping_hosts_performance: f32,
    pub ping_ips_performance: f32,
}

impl From<helpers::AsnKind> for AsnKind {
    fn from(value: helpers::AsnKind) -> Self {
        match value {
            helpers::AsnKind::Residential => AsnKind::Residential,
            helpers::AsnKind::Other => AsnKind::Other,
        }
    }
}

impl From<helpers::Asn> for Asn {
    fn from(location: helpers::Asn) -> Self {
        Asn {
            asn: location.asn,
            name: location.name,
            kind: location.kind.into(),
        }
    }
}

impl From<helpers::Location> for Location {
    fn from(location: helpers::Location) -> Self {
        Location {
            two_letter_iso_country_code: location.two_letter_iso_country_code,
            latitude: location.latitude,
            longitude: location.longitude,
            city: location.city,
            region: location.region,
            asn: location.asn.map(Into::into),
        }
    }
}

// impl From<nym_vpn_api_client::response::ScoreValue> for ScoreValue {
//     fn from(value: nym_vpn_api_client::response::ScoreValue) -> Self {
//         match value {
//             nym_vpn_api_client::response::ScoreValue::Offline => ScoreValue::Offline,
//             nym_vpn_api_client::response::ScoreValue::Low => ScoreValue::Low,
//             nym_vpn_api_client::response::ScoreValue::Medium => ScoreValue::Medium,
//             nym_vpn_api_client::response::ScoreValue::High => ScoreValue::High,
//         }
//     }
// }

// impl From<nym_vpn_api_client::response::DVpnGatewayPerformance> for Performance {
//     fn from(value: nym_vpn_api_client::response::DVpnGatewayPerformance) -> Self {
//         Performance {
//             last_updated_utc: value.last_updated_utc,
//             score: value.score.into(),
//             load: value.load.into(),
//             uptime_percentage_last_24_hours: value.uptime_percentage_last_24_hours,
//         }
//     }
// }

// impl From<nym_vpn_api_client::response::Probe> for Probe {
//     fn from(probe: nym_vpn_api_client::response::Probe) -> Self {
//         Probe {
//             last_updated_utc: probe.last_updated_utc,
//             outcome: ProbeOutcome::from(probe.outcome),
//         }
//     }
// }

impl From<Percent> for Score {
    fn from(percent: Percent) -> Self {
        let rounded_percent = percent.round_to_integer();
        if rounded_percent >= HIGH_SCORE_THRESHOLD {
            Score::High(rounded_percent)
        } else if rounded_percent >= MEDIUM_SCORE_THRESHOLD {
            Score::Medium(rounded_percent)
        } else if rounded_percent > LOW_SCORE_THRESHOLD {
            Score::Low(rounded_percent)
        } else {
            Score::None
        }
    }
}

// impl From<nym_vpn_api_client::response::ProbeOutcome> for ProbeOutcome {
//     fn from(outcome: nym_vpn_api_client::response::ProbeOutcome) -> Self {
//         ProbeOutcome {
//             as_entry: Entry::from(outcome.as_entry),
//             as_exit: outcome.as_exit.map(Exit::from),
//             wg: outcome.wg.map(WgProbeResults::from),
//         }
//     }
// }

// impl From<nym_vpn_api_client::response::Entry> for Entry {
//     fn from(entry: nym_vpn_api_client::response::Entry) -> Self {
//         Entry {
//             can_connect: entry.can_connect,
//             can_route: entry.can_route,
//         }
//     }
// }

// impl From<nym_vpn_api_client::response::Exit> for Exit {
//     fn from(exit: nym_vpn_api_client::response::Exit) -> Self {
//         Exit {
//             can_connect: exit.can_connect,
//             can_route_ip_v4: exit.can_route_ip_v4,
//             can_route_ip_external_v4: exit.can_route_ip_external_v4,
//             can_route_ip_v6: exit.can_route_ip_v6,
//             can_route_ip_external_v6: exit.can_route_ip_external_v6,
//         }
//     }
// }

// impl From<nym_vpn_api_client::response::WgProbeResults> for WgProbeResults {
//     fn from(results: nym_vpn_api_client::response::WgProbeResults) -> Self {
//         WgProbeResults {
//             can_register: results.can_register,
//             can_handshake: results.can_handshake,
//             can_resolve_dns: results.can_resolve_dns,
//             ping_hosts_performance: results.ping_hosts_performance,
//             ping_ips_performance: results.ping_ips_performance,
//         }
//     }
// }

// impl TryFrom<nym_vpn_api_client::response::NymDirectoryGateway> for Gateway {
//     type Error = Error;

//     fn try_from(gateway: nym_vpn_api_client::response::NymDirectoryGateway) -> Result<Self> {
//         let identity =
//             NodeIdentity::from_base58_string(&gateway.identity_key).map_err(|source| {
//                 Error::NodeIdentityFormattingError {
//                     identity: gateway.identity_key,
//                     source,
//                 }
//             })?;

//         let ipr_address = gateway
//             .ip_packet_router
//             .and_then(|ipr| IpPacketRouterAddress::try_from_base58_string(&ipr.address).ok());

//         let authenticator_address = gateway
//             .authenticator
//             .and_then(|auth| AuthAddress::try_from_base58_string(&auth.address).ok());

//         let hostname = gateway.entry.hostname;
//         let first_ip_address = gateway
//             .ip_addresses
//             .first()
//             .cloned()
//             .map(|ip| ip.to_string());
//         let host = hostname.or(first_ip_address);

//         Ok(Gateway {
//             identity,
//             moniker: gateway.name,
//             location: Some(gateway.location.into()),
//             ipr_address,
//             authenticator_address,
//             bridge_params: gateway.bridges,
//             last_probe: gateway.last_probe.map(Probe::from),
//             ips: gateway.ip_addresses,
//             host,
//             clients_ws_port: Some(gateway.entry.ws_port),
//             clients_wss_port: gateway.entry.wss_port,
//             mixnet_performance: Some(gateway.performance),
//             mixnet_score: Some(Score::from(gateway.performance)),
//             wg_performance: gateway.performance_v2.map(Performance::from),
//             version: gateway.build_information.map(|info| info.build_version),
//         })
//     }
// }

pub type NymNodeList = GatewayList;

#[derive(Debug, Clone)]
pub struct GatewayList {
    /// If None, then the list contains mixed types.
    gw_type: Option<GatewayType>,
    gateways: Vec<Gateway>,
}

impl GatewayList {
    pub fn new(gw_type: Option<GatewayType>, gateways: Vec<Gateway>) -> Self {
        GatewayList { gw_type, gateways }
    }

    // Returns a list of all locations of the gateways, including duplicates
    fn all_locations(&self) -> impl Iterator<Item = &Location> {
        self.gateways
            .iter()
            .filter_map(|gateway| gateway.location.as_ref())
    }

    pub fn all_countries(&self) -> Vec<Country> {
        self.all_locations()
            .cloned()
            .map(Country::from)
            .unique()
            .collect()
    }

    pub fn all_iso_codes(&self) -> Vec<String> {
        self.all_countries()
            .into_iter()
            .map(|country| country.iso_code().to_string())
            .collect()
    }

    pub fn filter(&self, filters: &[GatewayFilter]) -> Vec<Gateway> {
        self.gateways
            .iter()
            .filter(|gateway| gateway.matches_all_filters(self.gw_type, filters))
            .cloned()
            .collect()
    }

    pub fn node_with_identity(&self, identity: &NodeIdentity) -> Option<&Gateway> {
        // Not using self.filter() here as find() will stop at the first match
        self.gateways
            .iter()
            .find(|node| &node.identity() == identity)
    }

    pub fn gateway_with_identity(&self, identity: &NodeIdentity) -> Option<&Gateway> {
        self.node_with_identity(identity)
    }

    pub fn choose_random(&self, filters: &[GatewayFilter]) -> Option<Gateway> {
        self.filter(filters)
            .into_iter()
            .choose(&mut rand::thread_rng())
    }

    pub fn remove_gateway(&mut self, entry_gateway: &Gateway) {
        self.gateways
            .retain(|gateway| gateway.identity() != entry_gateway.identity());
    }

    pub fn gw_type(&self) -> Option<GatewayType> {
        self.gw_type
    }

    pub fn len(&self) -> usize {
        self.gateways.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gateways.is_empty()
    }

    pub fn into_exit_gateways(self) -> GatewayList {
        Self::new(self.gw_type, self.filter(&[GatewayFilter::Exit]))
    }

    pub fn into_vpn_gateways(self) -> GatewayList {
        Self::new(self.gw_type, self.filter(&[GatewayFilter::Vpn]))
    }

    pub fn into_inner(self) -> Vec<Gateway> {
        self.gateways
    }
}

impl IntoIterator for GatewayList {
    type Item = Gateway;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.gateways.into_iter()
    }
}

impl nym_client_core::init::helpers::ConnectableGateway for Gateway {
    #[allow(unconditional_recursion)]
    fn node_id(&self) -> NodeId {
        self.node_id()
    }

    fn identity(&self) -> NodeIdentity {
        self.identity()
    }

    fn clients_address(&self, _prefer_ipv6: bool) -> Option<String> {
        // This is a bit of a sharp edge, but temporary until we can remove Option from host
        // and tls port when we add these to the vpn API endpoints.
        Some(
            self.clients_address_tls()
                .or(self.clients_address_no_tls())
                .unwrap_or("ws://".to_string()),
        )
    }

    fn is_wss(&self) -> bool {
        self.clients_address_tls().is_some()
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, strum::EnumIter)]
pub enum GatewayType {
    MixnetEntry,
    MixnetExit,
    Wg,
}

impl fmt::Display for GatewayType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GatewayType::MixnetEntry => write!(f, "mixnet entry"),
            GatewayType::MixnetExit => write!(f, "mixnet exit"),
            GatewayType::Wg => write!(f, "vpn"),
        }
    }
}

impl From<helpers::GatewayType> for GatewayType {
    fn from(gateway_type: helpers::GatewayType) -> Self {
        match gateway_type {
            helpers::GatewayType::MixnetEntry => GatewayType::MixnetEntry,
            helpers::GatewayType::MixnetExit => GatewayType::MixnetExit,
            helpers::GatewayType::Wg => GatewayType::Wg,
        }
    }
}

impl From<GatewayType> for helpers::GatewayType {
    fn from(gateway_type: GatewayType) -> Self {
        match gateway_type {
            GatewayType::MixnetEntry => helpers::GatewayType::MixnetEntry,
            GatewayType::MixnetExit => helpers::GatewayType::MixnetExit,
            GatewayType::Wg => helpers::GatewayType::Wg,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GatewayFilter {
    MinScore(ScoreValue), // Mixnet or Wg score
    Country(String),      // Two-letter ISO country code
    Region(String),       // Region name
    Residential,          // Has a residential ASN
    Exit,                 // Has an IPR address
    Vpn,                  // Has an authenticator address
}
