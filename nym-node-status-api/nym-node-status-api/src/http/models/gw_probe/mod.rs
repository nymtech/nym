use serde::Deserialize;
use serde::Serialize;
use strum::EnumString;
use tracing::error;
use utoipa::ToSchema;

pub(crate) mod socks5_calc;
#[cfg(test)]
mod test;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct LastProbeResult {
    node: String,
    used_entry: String,
    outcome: ProbeOutcome,
}

use nym_gateway_probe::types::ProbeResult as ProbeResultLatest;

impl From<ProbeResultLatest> for LastProbeResult {
    fn from(value: ProbeResultLatest) -> Self {
        Self {
            node: value.node,
            used_entry: value.used_entry,
            outcome: value.outcome.into(),
        }
    }
}

impl LastProbeResult {
    pub(crate) fn deserialize_with_fallback(value: serde_json::Value) -> anyhow::Result<Self> {
        // first try matching latest struct from GW probe crate
        let mut probe_result = match serde_json::from_value::<ProbeResultLatest>(value.clone()) {
            Ok(probe_result) => probe_result.into(),
            // as a fallback, try parsing struct from this crate
            Err(_) => match serde_json::from_value::<Self>(value) {
                Ok(probe_result) => probe_result,
                Err(e) => {
                    error!("Failed to deserialize probe result: {e}");
                    return Err(e.into());
                }
            },
        };

        probe_result.outcome.wg = probe_result.outcome.wg.clone().map(|mut wg| {
            if wg.can_handshake.is_none() {
                wg.can_handshake = Some(wg.can_handshake_v4);
            }
            if wg.can_resolve_dns.is_none() {
                wg.can_resolve_dns = Some(wg.can_resolve_dns_v4);
            }
            if wg.ping_hosts_performance.is_none() {
                wg.ping_hosts_performance = Some(wg.ping_hosts_performance_v4);
            }
            if wg.ping_ips_performance.is_none() {
                wg.ping_ips_performance = Some(wg.ping_ips_performance_v4);
            }
            wg
        });

        Ok(probe_result)
    }

    pub(crate) fn outcome(self) -> ProbeOutcome {
        self.outcome
    }
}

/// gateway probe output returned on the API
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DvpnGwProbe {
    last_updated_utc: String,
    outcome: DvpnProbeOutcome,
}

impl DvpnGwProbe {
    pub fn from_outcome(outcome: DvpnProbeOutcome, last_updated_utc: String) -> Self {
        Self {
            last_updated_utc,
            outcome,
        }
    }

    pub fn can_route_entry(&self) -> bool {
        match &self.outcome.as_entry {
            Entry::Tested(entry_test_result) => entry_test_result.can_route,
            Entry::NotTested | Entry::EntryFailure => false,
        }
    }

    pub fn can_route_exit(&self) -> Option<bool> {
        self.outcome
            .as_exit
            .as_ref()
            .map(|outcome| outcome.can_route_ip_external_v4 && outcome.can_route_ip_external_v6)
    }
}

/// this structure is parsed on VPN API so it has some fields which must not be changed
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DvpnProbeOutcome {
    pub as_entry: Entry,
    pub as_exit: Option<Exit>,
    pub wg: Option<WgProbeResults>,
    pub socks5: Option<Socks5>,
    pub lp: Option<LpProbeResults>,
}

impl DvpnProbeOutcome {
    pub fn from_raw_probe_outcome(outcome: ProbeOutcome, score: ScoreValue) -> Self {
        let errors = outcome
            .socks5
            .clone()
            .and_then(|s| s.https_connectivity.errors);
        let can_proxy_https = outcome
            .socks5
            .map(|s| s.https_connectivity.https_success)
            .unwrap_or_else(|| match score {
                ScoreValue::Offline => false,
                ScoreValue::Low | ScoreValue::Medium | ScoreValue::High => true,
            });
        Self {
            as_entry: outcome.as_entry.clone(),
            as_exit: outcome.as_exit.clone(),
            wg: outcome.wg.clone(),
            socks5: Some(Socks5 {
                can_proxy_https,
                score,
                errors,
            }),
            lp: outcome.lp,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ProbeOutcome {
    pub as_entry: Entry,
    pub as_exit: Option<Exit>,
    pub wg: Option<WgProbeResults>,
    pub socks5: Option<Socks5ProbeResults>,
    pub lp: Option<LpProbeResults>,
}

use nym_gateway_probe::types::ProbeOutcome as ProbeOutcomeLatest;

impl From<ProbeOutcomeLatest> for ProbeOutcome {
    fn from(value: ProbeOutcomeLatest) -> Self {
        Self {
            as_entry: value.as_entry.into(),
            as_exit: value.as_exit.map(From::from),
            wg: value.wg.map(From::from),
            socks5: value.socks5.map(From::from),
            lp: value.lp.map(From::from),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
#[allow(clippy::enum_variant_names)]
pub enum Entry {
    Tested(EntryTestResult),
    NotTested,
    EntryFailure,
}

use nym_gateway_probe::types::Entry as EntryLatest;

impl From<EntryLatest> for Entry {
    fn from(value: EntryLatest) -> Self {
        match value {
            EntryLatest::Tested(entry_test_result) => Self::Tested(entry_test_result.into()),
            EntryLatest::NotTested => Self::NotTested,
            EntryLatest::EntryFailure => Self::EntryFailure,
        }
    }
}

use nym_gateway_probe::types::EntryTestResult as EntryTestResultLatest;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntryTestResult {
    pub can_connect: bool,
    pub can_route: bool,
}

impl From<EntryTestResultLatest> for EntryTestResult {
    fn from(value: EntryTestResultLatest) -> Self {
        Self {
            can_connect: value.can_connect,
            can_route: value.can_route,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Exit {
    pub can_connect: bool,
    pub can_route_ip_v4: bool,
    pub can_route_ip_external_v4: bool,
    pub can_route_ip_v6: bool,
    pub can_route_ip_external_v6: bool,
}

use nym_gateway_probe::types::Exit as ExitLatest;

impl From<ExitLatest> for Exit {
    fn from(value: ExitLatest) -> Self {
        Self {
            can_connect: value.can_connect,
            can_route_ip_v4: value.can_route_ip_v4,
            can_route_ip_external_v4: value.can_route_ip_external_v4,
            can_route_ip_v6: value.can_route_ip_v6,
            can_route_ip_external_v6: value.can_route_ip_external_v6,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct WgProbeResults {
    // mandatory fields
    pub can_register: bool,
    pub can_handshake: Option<bool>,
    pub can_resolve_dns: Option<bool>,
    pub ping_hosts_performance: Option<f32>,
    pub ping_ips_performance: Option<f32>,

    pub can_query_metadata_v4: Option<bool>,
    pub can_handshake_v4: bool,
    pub can_resolve_dns_v4: bool,
    pub ping_hosts_performance_v4: f32,
    pub ping_ips_performance_v4: f32,

    pub can_handshake_v6: bool,
    pub can_resolve_dns_v6: bool,
    pub ping_hosts_performance_v6: f32,
    pub ping_ips_performance_v6: f32,

    pub download_duration_sec_v4: u64,
    pub download_duration_milliseconds_v4: Option<u64>,
    pub downloaded_file_size_bytes_v4: Option<u64>,
    pub downloaded_file_v4: String,
    pub download_error_v4: String,

    pub download_duration_sec_v6: u64,
    pub download_duration_milliseconds_v6: Option<u64>,
    pub downloaded_file_size_bytes_v6: Option<u64>,
    pub downloaded_file_v6: String,
    pub download_error_v6: String,
}

use nym_gateway_probe::types::WgProbeResults as WgProbeResultsLatest;

use crate::http::models::Gateway;

impl From<WgProbeResultsLatest> for WgProbeResults {
    fn from(value: WgProbeResultsLatest) -> Self {
        Self {
            can_register: value.can_register,
            can_handshake: Some(value.can_handshake_v4),
            can_resolve_dns: Some(value.can_resolve_dns_v4),
            ping_hosts_performance: Some(value.ping_hosts_performance_v4),
            ping_ips_performance: Some(value.ping_ips_performance_v4),

            can_query_metadata_v4: Some(value.can_query_metadata_v4),
            can_handshake_v4: value.can_handshake_v4,
            can_resolve_dns_v4: value.can_resolve_dns_v4,
            ping_hosts_performance_v4: value.ping_hosts_performance_v4,
            ping_ips_performance_v4: value.ping_ips_performance_v4,

            can_handshake_v6: value.can_handshake_v6,
            can_resolve_dns_v6: value.can_resolve_dns_v6,
            ping_hosts_performance_v6: value.ping_hosts_performance_v6,
            ping_ips_performance_v6: value.ping_ips_performance_v6,

            download_duration_sec_v4: value.download_duration_sec_v4,
            download_duration_milliseconds_v4: Some(value.download_duration_milliseconds_v4),
            downloaded_file_size_bytes_v4: Some(value.downloaded_file_size_bytes_v4),
            downloaded_file_v4: value.downloaded_file_v4,
            download_error_v4: value.download_error_v4,

            download_duration_sec_v6: value.download_duration_sec_v6,
            download_duration_milliseconds_v6: Some(value.download_duration_milliseconds_v6),
            downloaded_file_size_bytes_v6: Some(value.downloaded_file_size_bytes_v6),
            downloaded_file_v6: value.downloaded_file_v6,
            download_error_v6: value.download_error_v6,
        }
    }
}

struct NodeScore {
    download_speed_score: f64,
    ping_ips_score: f64,
    mixnet_performance: f64,
}

impl NodeScore {
    // Weighted scoring: mixnet performance (40%), download speed (30%), ping performance (30%)
    fn calculate_weighted_score(&self) -> f64 {
        (self.mixnet_performance * 0.4)
            + (self.download_speed_score * 0.3)
            + (self.ping_ips_score * 0.3)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[derive(PartialEq)]
pub enum ScoreValue {
    Offline,
    Low,
    Medium,
    High,
}

/// calculates a visual score for the gateway using weighted metrics
pub(super) fn calc_gateway_visual_score(
    gateway: &Gateway,
    probe_outcome: &LastProbeResult,
) -> ScoreValue {
    let mixnet_performance = gateway.performance as f64 / 100.0;

    let node_score = probe_outcome
        .outcome
        .wg
        .as_ref()
        .map(|p| {
            let ping_ips_performance = p.ping_ips_performance_v4 as f64;

            let duration_sec =
                p.download_duration_milliseconds_v4
                    .unwrap_or_else(|| p.download_duration_sec_v4 * 1000) as f64
                    / 1000f64;

            // get the file size downloaded in bytes and convert to MB, or default to 1MB
            let file_size_mb =
                p.downloaded_file_size_bytes_v4.unwrap_or(1048576) as f64 / 1024f64 / 1024f64;
            let speed_mbps = file_size_mb / duration_sec;

            let file_download_score = if speed_mbps > 5.0 {
                1.0
            } else if speed_mbps > 2.0 {
                0.75
            } else if speed_mbps > 1.0 {
                0.5
            } else if speed_mbps > 0.5 {
                0.25
            } else {
                0.1
            };

            NodeScore {
                download_speed_score: file_download_score,
                ping_ips_score: ping_ips_performance,
                mixnet_performance,
            }
        })
        .unwrap_or(NodeScore {
            download_speed_score: 0.0,
            ping_ips_score: 0.0,
            mixnet_performance,
        });

    let weighted_score = node_score.calculate_weighted_score();

    if weighted_score > 0.75 {
        ScoreValue::High
    } else if weighted_score > 0.5 {
        ScoreValue::Medium
    } else if weighted_score > 0.1 {
        ScoreValue::Low
    } else {
        ScoreValue::Offline
    }
}

/// calculates a visual load score for the gateway
pub(super) fn calculate_load(probe_outcome: &LastProbeResult) -> ScoreValue {
    let score = probe_outcome
        .outcome
        .wg
        .clone()
        .map(|p| p.ping_ips_performance_v4 as f64)
        .unwrap_or(0f64);

    if score > 0.8 {
        ScoreValue::Low
    } else if score > 0.4 {
        ScoreValue::Medium
    } else {
        ScoreValue::High
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Socks5 {
    pub can_proxy_https: bool,
    pub score: ScoreValue,
    pub errors: Option<Vec<String>>,
}

use nym_gateway_probe::types::Socks5ProbeResults as Socks5ProbeResultsLatest;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Socks5ProbeResults {
    /// whether we could establish a SOCKS5 proxy connection
    pub can_connect_socks5: bool,

    /// HTTPS connectivity test
    pub https_connectivity: HttpsConnectivityResult,
}

impl From<Socks5ProbeResultsLatest> for Socks5ProbeResults {
    fn from(value: Socks5ProbeResultsLatest) -> Self {
        Self {
            can_connect_socks5: value.can_connect_socks5(),
            https_connectivity: value.https_connectivity().clone().into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct HttpsConnectivityResult {
    /// successfully completed HTTPS request
    https_success: bool,

    /// HTTPS status code received
    https_status_code: Option<u16>,

    /// average HTTPS request latency in milliseconds
    https_latency_ms: Option<u64>,

    /// among multiple endpoints available, list the one actually used
    endpoint_used: Option<String>,

    /// error message(s) (if any)
    errors: Option<Vec<String>>,
}

use nym_gateway_probe::types::HttpsConnectivityResult as HttpsConnectivityResultLatest;

impl From<HttpsConnectivityResultLatest> for HttpsConnectivityResult {
    fn from(value: HttpsConnectivityResultLatest) -> Self {
        Self {
            https_success: value.https_success(),
            https_status_code: value.https_status_code().cloned(),
            https_latency_ms: value.https_latency_ms().cloned(),
            endpoint_used: value.endpoint_used().cloned(),
            errors: value.errors().cloned(),
        }
    }
}

use nym_gateway_probe::types::LpProbeResults as LpProbeResultsLatest;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename = "lp")]
pub struct LpProbeResults {
    pub can_connect: bool,
    pub can_handshake: bool,
    pub can_register: bool,
    pub error: Option<String>,
}

impl From<LpProbeResultsLatest> for LpProbeResults {
    fn from(value: LpProbeResultsLatest) -> Self {
        Self {
            can_connect: value.can_connect,
            can_handshake: value.can_handshake,
            can_register: value.can_register,
            error: value.error,
        }
    }
}
