use dotenvy::dotenv;
use reqwest::{Client, Response};
use serde_json::Value;

#[allow(dead_code)]
#[allow(clippy::panic)]
pub fn test_client() -> Client {
    Client::new()
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub fn base_url() -> Result<String, String> {
    dotenv().ok();

    std::env::var("NYM_API")
        .map(|url| url.trim_end_matches('/').to_string())
        .map_err(|_| "Couldn't find NYM_API env var".to_string())
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub async fn make_request(url: &str) -> Result<Response, String> {
    let res = test_client()
        .get(url)
        .send()
        .await
        .map_err(|err| format!("Failed to send request to {url}: {err}"))?;

    if res.status().is_success() {
        Ok(res)
    } else {
        Err(format!("Expected 2xx but got {}", res.status()))
    }
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub async fn validate_json_response(res: Response) -> Result<Value, String> {
    if !res.status().is_success() {
        return Err(format!("Expected 2xx but got {}", res.status()));
    }

    res.json::<Value>()
        .await
        .map_err(|err| format!("Invalid JSON response: {err}"))
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub async fn get_any_node_id() -> Result<String, String> {
    let url = format!("{}/v1/nym-nodes/bonded", base_url()?);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("Failed to send request to {url}: {err}"))?;
    let json: Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse response as JSON: {err}"))?;

    let id = json
        .get("data")
        .and_then(|list| list.as_array())
        .and_then(|arr| arr.first())
        .and_then(|node| node.get("bond_information"))
        .and_then(|n| n.get("node_id"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    Ok(id.to_string())
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub async fn get_mixnode_node_id() -> Result<u64, String> {
    let url = format!("{}/v1/nym-nodes/described", base_url()?);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("Failed to send request to {url}: {err}"))?;
    let json: Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse response as JSON: {err}"))?;

    json.get("data")
        .and_then(|v| v.as_array())
        .ok_or("Expected 'data' to be an array".to_string())?
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
        .ok_or("Unable to find mixnode node id".to_string())
}

#[allow(dead_code)]
#[allow(clippy::panic)]
pub async fn get_gateway_identity_key() -> Result<String, String> {
    let url = format!("{}/v1/nym-nodes/described", base_url()?);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("Failed to send request to {url}: {err}"))?;
    let json: Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse response as JSON: {err}"))?;

    let key = json
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or("Expected 'data' to be an array".to_string())?
        .iter()
        .find(|entry| {
            entry
                .get("description")
                .and_then(|d| d.get("declared_role"))
                .and_then(|r| r.get("mixnode"))
                .and_then(|m| m.as_bool())
                .map(|is_mixnode| !is_mixnode)
                .unwrap_or(false)
        })
        .and_then(|node| {
            node.get("description")
                .and_then(|d| d.get("host_information"))
                .and_then(|h| h.get("keys"))
                .and_then(|k| k.get("ed25519"))
                .and_then(|v| v.as_str())
        })
        .ok_or("Unable to find gateway identity key with mixnode = false".to_string())?;

    Ok(key.to_string())
}
