use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub struct HttpsConnectivityTest {
    test_count: u64,
    mixnet_client_timeout: Duration,
    json_rpc_test_endpoints: Vec<String>,
}

impl HttpsConnectivityTest {
    pub fn new(
        test_count: u64,
        mixnet_client_timeout: u64,
        json_rpc_test_endpoints: Vec<String>,
    ) -> Self {
        Self {
            test_count: std::cmp::max(test_count, 1),
            mixnet_client_timeout: Duration::from_secs(mixnet_client_timeout),
            json_rpc_test_endpoints,
        }
    }

    pub async fn run_tests(
        self,
        socks5_url: String,
        failure_count_cutoff: usize,
    ) -> HttpsConnectivityResult {
        let proxy = match reqwest::Proxy::all(socks5_url) {
            Ok(p) => p,
            Err(e) => {
                return HttpsConnectivityResult::with_error(
                    format!("Failed to create proxy: {e}",),
                );
            }
        };

        let client = match reqwest::Client::builder()
            .proxy(proxy)
            .timeout(self.mixnet_client_timeout)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return HttpsConnectivityResult::with_error(format!(
                    "Failed to build HTTP client: {e}",
                ));
            }
        };

        let mut results = Vec::new();

        for i in 1..=self.test_count {
            info!("Running test {}/{}", i, self.test_count);
            let interim_res = self.perform_https_request(&client).await;

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
            if unsuccessful > failure_count_cutoff {
                warn!("Too many failed runs: returning early...");
                break;
            }
        }

        let final_result = HttpsConnectivityResult::from_results(results);
        info!("AVG latency (in ms): {:?}", final_result.https_latency_ms);
        final_result
    }

    async fn perform_https_request(&self, client: &reqwest::Client) -> SingleHttpsTestResult {
        use tokio::time::Instant;

        let start = Instant::now();
        let mut error_msg = String::new();

        for endpoint in self.json_rpc_test_endpoints.iter() {
            info!(
                "Testing against {} with timeout {}s",
                endpoint,
                self.mixnet_client_timeout.as_secs()
            );
            match client
                .post(endpoint)
                .timeout(self.mixnet_client_timeout)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .json(&JsonRpcRequestBody::eth_chainid())
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
            {
                Ok(response) => {
                    let elapsed = start.elapsed();
                    let status = response.status();

                    if status.is_success() {
                        // Deserialize body into JsonRpcResponse
                        match response.json::<JsonRpcResponse>().await {
                            Ok(JsonRpcResponse::Ok { .. }) => {
                                debug!(
                                    "HTTPS test completed: status={}, latency={}ms",
                                    status.as_u16(),
                                    elapsed.as_millis()
                                );
                                return SingleHttpsTestResult {
                                    success: true,
                                    status_code: Some(status.as_u16()),
                                    latency_ms: Some(elapsed.as_millis() as u64),
                                    endpoint_used: Some(endpoint.to_string()),
                                    error: None,
                                };
                            }
                            Ok(JsonRpcResponse::Err { error, .. }) => {
                                warn!("JSON-RPC error: {} (code: {})", error.message, error.code);
                                error_msg = format!("JSON-RPC error: {}", error.message);
                            }
                            Err(e) => {
                                error!("Failed to parse JSON-RPC response: {}", e);
                                error_msg = format!("Failed to parse JSON-RPC response: {e}");
                            }
                        }
                    } else {
                        error_msg = format!("HTTP error status: {}", status.as_u16());
                    }
                }
                Err(e) => {
                    error!("HTTPS request failed: {}", e);

                    error_msg = format!("HTTPS request failed: {}", e);
                }
            }
        }

        SingleHttpsTestResult {
            success: false,
            status_code: None,
            latency_ms: None,
            endpoint_used: None,
            error: Some(error_msg),
        }
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
    error: Option<String>,
}

impl HttpsConnectivityResult {
    pub fn with_error(error: impl Into<String>) -> Self {
        Self {
            https_success: false,
            https_status_code: None,
            https_latency_ms: None,
            endpoint_used: None,
            error: Some(error.into()),
        }
    }

    fn from_results(results: Vec<SingleHttpsTestResult>) -> Self {
        let successes: Vec<_> = results.iter().filter(|r| r.success).collect();
        let errors: Vec<_> = results.iter().filter_map(|r| r.error.as_ref()).collect();

        if successes.is_empty() {
            return Self::with_error(
                errors
                    .into_iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }

        // average latency from successful runs
        let total_latency: u64 = successes.iter().filter_map(|r| r.latency_ms).sum();
        let avg_latency = total_latency / successes.len() as u64;

        // use the last successful result for status_code and endpoint
        let last_success = successes.last().unwrap();

        Self {
            https_success: true,
            https_status_code: last_success.status_code,
            https_latency_ms: Some(avg_latency),
            endpoint_used: last_success.endpoint_used.clone(),
            error: if errors.is_empty() {
                None
            } else {
                Some(
                    errors
                        .into_iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            },
        }
    }
}

/// https://www.jsonrpc.org/specification
#[derive(Serialize)]
struct JsonRpcRequestBody {
    // A String specifying the version of the JSON-RPC protocol. MUST be exactly "2.0".
    jsonrpc: String,
    method: String,
    // A Structured value that holds the parameter values to be used during the invocation of the method. This member MAY be omitted.
    params: serde_json::Value,
    // The Server MUST reply with the same value in the Response object if included.
    // This member is used to correlate the context between the two objects.
    id: i64,
}

impl JsonRpcRequestBody {
    /// Very simple endpoint that requires no dynamic input
    ///
    /// https://ethereum.org/developers/docs/apis/json-rpc/#eth_chainId
    pub fn eth_chainid() -> Self {
        Self {
            jsonrpc: String::from("2.0"),
            method: String::from("eth_chainId"),
            params: serde_json::json!([]),
            id: rand::thread_rng().r#gen(),
        }
    }

    /// Create an eth_getBlockByNumber request with invalid params for testing error responses
    #[cfg(test)]
    pub fn eth_get_block_by_number_invalid() -> Self {
        Self {
            jsonrpc: String::from("2.0"),
            method: String::from("eth_getBlockByNumber"),
            // Invalid params: should be [blockNumber, boolean] but we pass garbage
            params: serde_json::json!(["invalid_block_number"]),
            id: rand::thread_rng().r#gen(),
        }
    }
}

// dead code: we need these fields for deserialization, even if we don't read them explicitly
#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(untagged)]
enum JsonRpcResponse {
    Ok {
        jsonrpc: String,
        // have to use opaque Value because spec say this might be string, number or null (we don't care either way)
        id: serde_json::Value,
        // we don't really care for the exact result, just whether the response is OK or error
        result: serde_json::Value,
    },
    Err {
        jsonrpc: String,
        // have to use opaque Value because spec say this might be string, number or null (we don't care either way)
        id: serde_json::Value,
        error: JsonRpcError,
    },
}

// dead code: we need these fields for deserialization, even if we don't read them explicitly
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const JSON_RPC_ENDPOINT: &str = "https://cloudflare-eth.com";

    #[tokio::test]
    async fn test_eth_chainid_returns_ok_response() {
        let client = reqwest::Client::new();
        let response = client
            .post(JSON_RPC_ENDPOINT)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&JsonRpcRequestBody::eth_chainid())
            .send()
            .await
            .expect("Failed to send request");

        assert!(response.status().is_success());

        let json_response: JsonRpcResponse =
            response.json().await.expect("Failed to parse response");

        assert!(
            matches!(json_response, JsonRpcResponse::Ok { .. }),
            "Expected Ok variant for eth_chainId"
        );
    }

    #[tokio::test]
    async fn test_eth_get_block_by_number_invalid_returns_error_response() {
        let client = reqwest::Client::new();
        let response = client
            .post(JSON_RPC_ENDPOINT)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&JsonRpcRequestBody::eth_get_block_by_number_invalid())
            .send()
            .await
            .expect("Failed to send request");

        assert!(response.status().is_success()); // HTTP 200 but JSON-RPC error

        let json_response: JsonRpcResponse =
            response.json().await.expect("Failed to parse response");

        assert!(
            matches!(json_response, JsonRpcResponse::Err { .. }),
            "Expected Err variant for invalid params"
        );
    }
}
