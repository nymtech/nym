use anyhow::{Context, bail};
use rand::Rng;
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

use crate::common::socks5_test::SingleHttpsTestResult;

pub struct JsonRpcClient {
    client: reqwest::Client,
    client_timeout: Duration,
    test_endpoints: Vec<String>,
}

impl JsonRpcClient {
    pub fn new(
        client_timeout: u64,
        proxy: Option<Proxy>,
        test_endpoints: Vec<String>,
    ) -> anyhow::Result<Self> {
        let mut builder = reqwest::Client::builder().timeout(Duration::from_secs(client_timeout));

        if let Some(proxy) = proxy {
            builder = builder.proxy(proxy);
        }
        let client = builder.build()?;

        Ok(Self {
            client_timeout: Duration::from_secs(client_timeout),
            test_endpoints,
            client,
        })
    }

    pub(super) async fn https_request_with_fallbacks(&self) -> SingleHttpsTestResult {
        let mut error_msg = Vec::new();

        // endpoints are used as fallbacks: in case of success, return early
        for endpoint in self.test_endpoints.iter() {
            info!(
                "Testing against {} with timeout {}s",
                endpoint,
                self.client_timeout.as_secs()
            );
            let start = Instant::now();

            let res = self.eth_chainid(endpoint).await;
            let elapsed = start.elapsed();
            match res {
                Ok((status, JsonRpcResponse::Ok { .. })) => {
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
                Ok((_, JsonRpcResponse::Err { error, .. })) => {
                    warn!("JSON-RPC error: {} (code: {})", error.message, error.code);
                    error_msg.push(format!("JSON-RPC error: {}", error.message));
                }
                Err(e) => {
                    error_msg.push(e.to_string());
                    error!("{}", &e);
                }
            }
        }

        SingleHttpsTestResult {
            success: false,
            status_code: None,
            latency_ms: None,
            endpoint_used: Some(self.test_endpoints.join(",")),
            error: Some(error_msg.join(",")),
        }
    }

    async fn eth_chainid(
        &self,
        endpoint: &str,
    ) -> anyhow::Result<(reqwest::StatusCode, JsonRpcResponse)> {
        match self
            .client
            .post(endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&JsonRpcRequestBody::eth_chainid())
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
        {
            Ok(response) => {
                let status = response.status();
                let response_text = response
                    .text()
                    .await
                    .context("Failed to extract response text")
                    .unwrap_or_else(|e| e.to_string());
                if status.is_success() {
                    // Deserialize body into JsonRpcResponse
                    serde_json::from_str::<JsonRpcResponse>(&response_text)
                        .map(|res| (status, res))
                        .map_err(From::from)
                } else {
                    bail!(
                        "HTTP error: {}\n{}",
                        status.as_u16(),
                        // truncate for logs in case response is too long
                        response_text.chars().take(200).collect::<String>()
                    );
                }
            }
            Err(e) => {
                let status = e
                    .status()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "no HTTP status".to_string());
                error!("HTTPS request failed: {}", e);
                bail!("HTTPS request failed: {} ({})", e, status);
            }
        }
    }

    pub async fn ensure_endpoint_works(&self) -> anyhow::Result<()> {
        let mut any_works = false;
        for endpoint in self.test_endpoints.iter() {
            if let Err(err) = self.eth_chainid(endpoint).await {
                warn!("Endpoint {endpoint} error: {err}");
            } else {
                any_works = true;
            }
        }

        if any_works {
            Ok(())
        } else {
            bail!("None of the endpoints are valid, see logs");
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
