use serde::{Deserialize, Serialize};
use tracing::{info, warn};

pub(crate) use json_rpc_client::JsonRpcClient;

mod json_rpc_client;

pub struct HttpsConnectivityTest {
    test_count: u64,
    failure_count_cutoff: usize,
    client: JsonRpcClient,
}

impl HttpsConnectivityTest {
    pub fn new(
        test_count: u64,
        mixnet_client_timeout: u64,
        failure_count_cutoff: usize,
        json_rpc_test_endpoints: Vec<String>,
        socks5_proxy_url: String,
    ) -> anyhow::Result<Self> {
        let proxy = reqwest::Proxy::all(socks5_proxy_url)
            .map_err(|e| anyhow::anyhow!("Failed to create proxy: {}", e))?;
        let client =
            JsonRpcClient::new(mixnet_client_timeout, Some(proxy), json_rpc_test_endpoints)?;
        let res = Self {
            test_count: std::cmp::max(test_count, 1),
            failure_count_cutoff,
            client,
        };

        Ok(res)
    }

    pub async fn run_tests(self) -> HttpsConnectivityResult {
        let mut results = Vec::new();

        for i in 1..=self.test_count {
            info!("Running test {}/{}", i, self.test_count);
            let interim_res = self.client.https_request_with_fallbacks().await;

            if interim_res.success {
                info!(
                    "{}/{} latency: {}ms",
                    i,
                    self.test_count,
                    interim_res.latency_ms.unwrap_or(0)
                );
            }

            results.push(interim_res);

            // early exit
            let unsuccessful = results.iter().filter(|r| !r.success).count();
            if unsuccessful > self.failure_count_cutoff {
                warn!("Too many failed runs: returning early...");
                break;
            }
        }

        let final_result = HttpsConnectivityResult::from_results(results);
        info!("AVG latency (in ms): {:?}", final_result.https_latency_ms);
        final_result
    }
}

/// single HTTPS test attempt
struct SingleHttpsTestResult {
    success: bool,
    status_code: Option<u16>,
    latency_ms: Option<u64>,
    endpoint_used: Option<String>,
    error: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

impl HttpsConnectivityResult {
    pub fn with_errors(errors: Vec<String>) -> Self {
        Self {
            https_success: false,
            https_status_code: None,
            https_latency_ms: None,
            endpoint_used: None,
            errors: Some(errors),
        }
    }

    fn from_results(results: Vec<SingleHttpsTestResult>) -> Self {
        let (successes, errors): (Vec<SingleHttpsTestResult>, Vec<SingleHttpsTestResult>) =
            results.into_iter().partition(|r| r.success);
        let errors = errors
            .into_iter()
            .map(|r| r.error)
            .collect::<Option<Vec<_>>>()
            // partition above guarantees this vec is non-empty
            .unwrap_or_default();

        // use the last successful result for status_code and endpoint
        // this works as an empty check as well: if there is no last success, array must be empty hence only errors are present
        let Some(last_success) = successes.last() else {
            return Self::with_errors(errors);
        };

        // average latency from successful runs
        let mut successes_count = 0;
        let total_latency: u64 = successes
            .iter()
            .filter_map(|r| {
                successes_count += 1;
                r.latency_ms
            })
            .sum();
        let avg_latency = total_latency / successes_count as u64;

        Self {
            https_success: true,
            https_status_code: last_success.status_code,
            https_latency_ms: Some(avg_latency),
            endpoint_used: last_success.endpoint_used.clone(),
            // even in case of success, some errors were possible
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
        }
    }

    pub fn https_success(&self) -> bool {
        self.https_success
    }

    pub fn https_status_code(&self) -> Option<&u16> {
        self.https_status_code.as_ref()
    }

    pub fn https_latency_ms(&self) -> Option<&u64> {
        self.https_latency_ms.as_ref()
    }

    pub fn endpoint_used(&self) -> Option<&String> {
        self.endpoint_used.as_ref()
    }

    pub fn errors(&self) -> Option<&Vec<String>> {
        self.errors.as_ref()
    }
}
