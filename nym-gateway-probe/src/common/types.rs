use nym_connection_monitor::ConnectionStatusEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use super::bandwidth_helpers::{AttachedTicket, AttachedTicketMaterials};
pub use super::socks5_test::HttpsConnectivityResult;
pub use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub node: String,
    pub used_entry: String,
    pub outcome: ProbeOutcome,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeOutcome {
    pub as_entry: Entry,
    pub as_exit: Option<Exit>,
    pub socks5: Option<Socks5ProbeResults>,
    pub wg: Option<WgProbeResults>,
    pub lp: Option<LpProbeResults>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename = "wg")]
pub struct WgProbeResults {
    pub can_register: bool,

    pub can_query_metadata_v4: bool,
    pub can_handshake_v4: bool,
    pub can_resolve_dns_v4: bool,
    pub ping_hosts_performance_v4: f32,
    pub ping_ips_performance_v4: f32,

    pub can_handshake_v6: bool,
    pub can_resolve_dns_v6: bool,
    pub ping_hosts_performance_v6: f32,
    pub ping_ips_performance_v6: f32,

    pub download_duration_sec_v4: u64,
    pub download_duration_milliseconds_v4: u64,
    pub downloaded_file_size_bytes_v4: u64,
    pub downloaded_file_v4: String,
    pub download_error_v4: String,

    pub download_duration_sec_v6: u64,
    pub downloaded_file_size_bytes_v6: u64,
    pub download_duration_milliseconds_v6: u64,
    pub downloaded_file_v6: String,
    pub download_error_v6: String,

    /// port → open/closed from exit policy check (if requested)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_check_results: Option<HashMap<String, bool>>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename = "lp")]
pub struct LpProbeResults {
    pub can_connect: bool,
    pub can_handshake: bool,
    pub can_register: bool,
    pub error: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::enum_variant_names)]
pub enum Entry {
    Tested(EntryTestResult),
    NotTested,
    EntryFailure,
}

impl From<EntryTestResult> for Entry {
    fn from(value: EntryTestResult) -> Self {
        Entry::Tested(value)
    }
}

impl Entry {
    pub fn fail_to_connect() -> Self {
        EntryTestResult {
            can_connect: false,
            can_route: false,
        }
        .into()
    }

    pub fn fail_to_route() -> Self {
        EntryTestResult {
            can_connect: true,
            can_route: false,
        }
        .into()
    }

    pub fn success() -> Self {
        EntryTestResult {
            can_connect: true,
            can_route: true,
        }
        .into()
    }
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryTestResult {
    pub can_connect: bool,
    pub can_route: bool,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exit {
    pub can_connect: bool,
    pub can_route_ip_v4: bool,
    pub can_route_ip_external_v4: bool,
    pub can_route_ip_v6: bool,
    pub can_route_ip_external_v6: bool,
}

impl Exit {
    pub fn fail_to_connect() -> Self {
        Self {
            can_connect: false,
            can_route_ip_v4: false,
            can_route_ip_external_v4: false,
            can_route_ip_v6: false,
            can_route_ip_external_v6: false,
        }
    }

    pub fn from_ping_replies(replies: &IpPingReplies) -> Self {
        Self {
            can_connect: true,
            can_route_ip_v4: replies.ipr_tun_ip_v4,
            can_route_ip_external_v4: replies.external_ip_v4,
            can_route_ip_v6: replies.ipr_tun_ip_v6,
            can_route_ip_external_v6: replies.external_ip_v6,
        }
    }
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Socks5ProbeResults {
    /// whether we could establish a SOCKS5 proxy connection
    can_connect_socks5: bool,

    /// HTTPS connectivity test
    https_connectivity: HttpsConnectivityResult,
}

impl Socks5ProbeResults {
    pub fn with_http_result(https_connectivity: HttpsConnectivityResult) -> Self {
        Self {
            can_connect_socks5: true,
            https_connectivity,
        }
    }

    pub fn error_before_connecting(error: impl Into<String>) -> Self {
        Self {
            can_connect_socks5: false,
            https_connectivity: HttpsConnectivityResult::with_errors(vec![error.into()]),
        }
    }

    pub fn error_after_connecting(error: impl Into<String>) -> Self {
        Self {
            can_connect_socks5: true,
            https_connectivity: HttpsConnectivityResult::with_errors(vec![error.into()]),
        }
    }

    pub fn can_connect_socks5(&self) -> bool {
        self.can_connect_socks5
    }

    pub fn https_connectivity(&self) -> &HttpsConnectivityResult {
        &self.https_connectivity
    }

    #[cfg(feature = "test-utils")]
    pub fn from_dummy_values(
        can_connect_socks5: bool,
        https_connectivity: HttpsConnectivityResult,
    ) -> Self {
        Self {
            can_connect_socks5,
            https_connectivity,
        }
    }
}

/// Output of the `run-ports` subcommand — per-port TCP reachability through
/// the WG exit tunnel, without the full probe outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortCheckResult {
    pub gateway: String,
    pub can_register: bool,
    pub port_check_target: String,
    /// port → open/closed
    pub ports: HashMap<String, bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct IpPingReplies {
    pub ipr_tun_ip_v4: bool,
    pub ipr_tun_ip_v6: bool,
    pub external_ip_v4: bool,
    pub external_ip_v6: bool,
}

impl IpPingReplies {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_event(&mut self, event: &ConnectionStatusEvent) {
        match event {
            ConnectionStatusEvent::MixnetSelfPing => {}
            ConnectionStatusEvent::Icmpv4IprTunDevicePingReply => self.ipr_tun_ip_v4 = true,
            ConnectionStatusEvent::Icmpv6IprTunDevicePingReply => self.ipr_tun_ip_v6 = true,
            ConnectionStatusEvent::Icmpv4IprExternalPingReply => self.external_ip_v4 = true,
            ConnectionStatusEvent::Icmpv6IprExternalPingReply => self.external_ip_v6 = true,
        }
    }
}
