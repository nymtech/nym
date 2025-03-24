
use reqwest::Client;
use serde_json::Value;

pub fn test_client() -> Client {
    Client::new()
}

pub fn base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "https://sandbox-nym-api1.nymtech.net/api".into())
}

pub async fn get_any_node_id() -> String {
    let url = format!("{}/v1/nym-nodes/bonded", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json: Value = res.json().await.unwrap();

    json.get("data")
        .and_then(|list| list.as_array())
        .and_then(|arr| arr.first())
        .and_then(|node| node.get("bond_information"))
        .and_then(|n| n.get("node_id"))
        .and_then(|v| v.as_u64())
        .map(|id| id.to_string())
        .unwrap_or_else(|| "INVALID_ID".into())
        .to_string()
}

pub async fn get_mixnode_node_id() -> u64 {
    let url = format!("{}/v1/nym-nodes/described", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json: Value = res.json().await.unwrap();

    json.get("data")
        .and_then(|v| v.as_array())
        .expect("Expected 'data' to be an array")
        .iter()
        .find(|entry| {
            entry
                .get("description")
                .and_then(|d| d.get("declared_role"))
                .and_then(|r| r.get("mixnode"))
                .and_then(|m| m.as_bool())
                .map(|is_mixnode| is_mixnode) 
                .unwrap_or(true)
        })
        .and_then(|node| {
            node.get("node_id")
                .and_then(|v| v.as_u64())
        })
        .expect("Unable to find mixnode node id")
}


pub async fn get_gateway_identity_key() -> String {
    let url = format!("{}/v1/nym-nodes/described", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json: Value = res.json().await.unwrap();

    json.get("data")
        .and_then(|v| v.as_array())
        .expect("Expected 'data' to be an array")
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
