use dotenv::dotenv;
use reqwest::Client;
use reqwest::Response;
use serde_json::Value;

pub fn test_client() -> Client {
    Client::new()
}

#[allow(clippy::panic)]
pub fn base_url() -> String {
    dotenv().ok();

    std::env::var("NYM_API").unwrap_or_else(|_err| {
        std::env::var("NYM_API")
            .unwrap_or_else(|_| panic!("Couldn't find NYM_API env var"))
            .trim_end_matches('/')
            .to_string()
    })
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub async fn validate_json_response(res: Response) -> Value {
    assert!(
        res.status().is_success(),
        "Expected 2xx but got {}",
        res.status()
    );
    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Invalid JSON response: {}", err));
    json
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub async fn get_any_node_id() -> String {
    let url = format!("{}/v1/nym-nodes/bonded", base_url());
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));

    json.get("data")
        .and_then(|list| list.as_array())
        .and_then(|arr| arr.first())
        .and_then(|node| node.get("bond_information"))
        .and_then(|n| n.get("node_id"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        .to_string()
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub async fn get_mixnode_node_id() -> u64 {
    let url = format!("{}/v1/nym-nodes/described", base_url());
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));

    json.get("data")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("Expected 'data' to be an array"))
        .iter()
        .find(|entry| {
            entry
                .get("description")
                .and_then(|d| d.get("declared_role"))
                .and_then(|r| r.get("mixnode"))
                .and_then(|m| m.as_bool())
                .unwrap_or(true)
        })
        .and_then(|node| node.get("node_id").and_then(|v| v.as_u64()))
        .expect("Unable to find mixnode node id")
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub async fn get_gateway_identity_key() -> String {
    let url = format!("{}/v1/nym-nodes/described", base_url());
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));

    json.get("data")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("Expected 'data' to be an array"))
        .iter()
        .find(|entry| {
            entry
                .get("description")
                .and_then(|d| d.get("declared_role"))
                .and_then(|r| r.get("mixnode"))
                .and_then(|m| m.as_bool())
                .map(|is_mixnode| !is_mixnode) // we want gateways, not mixnodes
                .unwrap_or(false)
        })
        .and_then(|node| {
            node.get("description")
                .and_then(|d| d.get("host_information"))
                .and_then(|h| h.get("keys"))
                .and_then(|k| k.get("ed25519"))
                .and_then(|v| v.as_str())
        })
        .expect("Unable to find gateway identity key with mixnode = false")
        .to_string()
}
