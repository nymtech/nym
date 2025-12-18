use std::time::Duration;

use nym_connection_monitor::ConnectionStatusEvent;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub node: String,
    pub used_entry: String,
    pub outcome: ProbeOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeOutcome {
    pub as_entry: Entry,
    pub as_exit: Option<Exit>,
    pub socks5: Option<Socks5ProbeResults>,
    pub wg: Option<WgProbeResults>,
}

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
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryTestResult {
    pub can_connect: bool,
    pub can_route: bool,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Socks5ProbeResults {
    /// whether we could establish a SOCKS5 proxy connection
    pub can_connect_socks5: bool,

    /// HTTPS connectivity test
    pub https_connectivity: HttpsConnectivityResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpsConnectivityResult {
    /// successfully completed HTTPS request
    https_success: bool,

    /// HTTPS status code received
    https_status_code: Option<u16>,

    /// average HTTPS request latency in milliseconds
    https_latency_ms: Option<u64>,

    /// error message(s) (if any)
    error: Option<String>,
}

impl HttpsConnectivityResult {
    pub fn with_error(error: impl Into<String>) -> Self {
        Self {
            https_success: false,
            https_status_code: None,
            https_latency_ms: None,
            error: Some(error.into()),
        }
    }
}

pub struct HttpsConnectivityTest {
    test_count: usize,
}

/// currently we test against this endpoint
/// https://www.quicknode.com/docs/ethereum/web3_clientVersion
const TARGET_URL: &str = "https://docs-demo.quiknode.pro";
const POST_BODY: &str = r#"{"jsonrpc":"2.0","method":"web3_clientVersion","params":[],"id":1}"#;
const MIXNET_TIMEOUT: Duration = Duration::from_secs(60);

impl HttpsConnectivityTest {
    pub fn new(test_count: usize) -> Self {
        Self {
            test_count: std::cmp::max(test_count, 1),
        }
    }

    pub async fn run_tests(self, socks5_url: String) -> HttpsConnectivityResult {
        let mut result = HttpsConnectivityResult::default();

        let proxy = match reqwest::Proxy::all(socks5_url) {
            Ok(p) => p,
            Err(e) => {
                result.error = Some(format!("Failed to create proxy: {}", e));
                return result;
            }
        };

        let client = match reqwest::Client::builder()
            .proxy(proxy)
            // longer timeout for mixnet
            .timeout(MIXNET_TIMEOUT)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                result.error = Some(format!("Failed to build HTTP client: {}", e));
                return result;
            }
        };

        let mut successful_runs = 0;
        for i in 0..self.test_count {
            info!("Running test {}/{}", i + 1, self.test_count);
            let interim_res = self.perform_https_request(&client).await;
            if interim_res.https_success
                && let Some(latency_ms) = interim_res.https_latency_ms
            {
                successful_runs += 1;
                result.https_latency_ms = Some(
                    result
                        .https_latency_ms
                        .map_or(latency_ms, |existing| existing + latency_ms),
                );
                result.https_success = true;
                result.https_status_code = interim_res.https_status_code;
                info!("{}/{} latency: {}ms", i + 1, self.test_count, latency_ms);
            } else if let Some(new_error) = interim_res.error {
                result.error = result
                    .error
                    .map(|existing| format!("{},{}", existing, new_error));
            }
        }
        result.https_latency_ms = result
            .https_latency_ms
            .map(|latency| latency / successful_runs);
        info!(
            "AVG latency over {} runs (in ms): {:?}",
            successful_runs, result.https_latency_ms
        );

        result
    }

    async fn perform_https_request(&self, client: &reqwest::Client) -> HttpsConnectivityResult {
        use tokio::time::Instant;

        let mut result = HttpsConnectivityResult::default();
        let start = Instant::now();
        match tokio::time::timeout(
            MIXNET_TIMEOUT,
            client
                .post(TARGET_URL)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(POST_BODY)
                .send(),
        )
        .await
        {
            Ok(Ok(response)) => {
                let elapsed = start.elapsed();
                let status = response.status();
                result.https_success = status.is_success();
                result.https_status_code = Some(status.as_u16());
                result.https_latency_ms = Some(elapsed.as_millis() as u64);
                debug!(
                    "HTTPS test completed: status={}, latency={}ms",
                    status.as_u16(),
                    elapsed.as_millis()
                );
            }
            Ok(Err(e)) => {
                warn!("HTTPS request failed: {}", e);
                if result.error.is_none() {
                    result.error = Some(format!("HTTPS request failed: {}", e));
                }
            }
            Err(_) => {
                warn!(
                    "HTTPS request timed out after {}s",
                    MIXNET_TIMEOUT.as_secs()
                );
                if result.error.is_none() {
                    result.error = Some("HTTPS request timed out".to_string());
                }
            }
        }

        result
    }
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
