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

    pub async fn run_tests(self, socks5_url: String) -> HttpsConnectivityResult {
        let mut result = HttpsConnectivityResult::default();

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

        let mut successful_runs = 0u64;
        for i in 1..self.test_count + 1 {
            info!("Running test {}/{}", i, self.test_count);
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
                info!("{}/{} latency: {}ms", i, self.test_count, latency_ms);
            } else if let Some(new_error) = interim_res.error {
                result.error = Some(result.error.map_or(new_error.clone(), |existing| {
                    format!("{},{}", existing, new_error)
                }))
            }

            // too many failed runs: return early
            let unsuccessful_runs = i - successful_runs;
            if successful_runs < 2 && unsuccessful_runs > 2 {
                // if < 2 runs, we don't have to calculate average before returning
                warn!("Too many failed runs: returning early...");
                return result;
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

        // TODO dz instead of initializing a mutable default, then mutating fields, use constructors for outcome
        let start = Instant::now();
        let mut error_msg = String::new();

        // TODO dz utilize others as fallback
        // let endpoint = self.json_rpc_test_endpoints.first().unwrap();
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
                                let res = HttpsConnectivityResult::success(
                                    status.as_u16(),
                                    elapsed.as_millis() as u64,
                                    endpoint.to_string(),
                                );
                                debug!(
                                    "HTTPS test completed: status={}, latency={}ms",
                                    status.as_u16(),
                                    elapsed.as_millis()
                                );
                                return res;
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

        HttpsConnectivityResult::with_error(error_msg)
    }
}

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

    pub fn success(status_code: u16, latency: u64, endpoint_used: String) -> Self {
        Self {
            https_success: true,
            https_status_code: Some(status_code),
            https_latency_ms: Some(latency),
            endpoint_used: Some(endpoint_used),
            error: None,
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
