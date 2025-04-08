mod utils;
use crate::utils::{base_url, test_client};
use serde_json::Value;

#[tokio::test]
async fn test_get_skimmed_nodes_active() {
    let url = format!("{}/v1/unstable/nym-nodes/skimmed/active", base_url());
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    assert!(
        res.status().is_success(),
        "Expected 2xx but got {}",
        res.status()
    );

    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));
    let data = json
        .get("nodes")
        .and_then(|r| r.get("data"))
        .expect("Missing 'data' field");

    assert!(data.is_array(), "Expected 'data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one node to appear"
    );
}

#[tokio::test]
async fn test_get_skimmed_active_mixnodes() {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/mixnodes/active",
        base_url()
    );
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    assert!(
        res.status().is_success(),
        "Expected 2xx but got {}",
        res.status()
    );

    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .expect("Missing 'current_epoch_id'");

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be an array"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
}

#[tokio::test]
async fn test_get_skimmed_all_mixnodes() {
    let url = format!("{}/v1/unstable/nym-nodes/skimmed/mixnodes/all", base_url());
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    assert!(
        res.status().is_success(),
        "Expected 2xx but got {}",
        res.status()
    );

    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .expect("Expected a value for 'current_epoch_id'");

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be an array"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
}

#[tokio::test]
async fn test_get_skimmed_active_exit_gateways() {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/exit-gateways/active",
        base_url()
    );
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    assert!(
        res.status().is_success(),
        "Expected 2xx but got {}",
        res.status()
    );

    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .expect("Expected a value for 'current_epoch_id'");

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be an array"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
}

#[tokio::test]
async fn test_get_skimmed_all_exit_gateways() {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/exit-gateways/all",
        base_url()
    );
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    assert!(
        res.status().is_success(),
        "Expected 2xx but got {}",
        res.status()
    );

    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .expect("Expected a value for 'current_epoch_id'");

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be an array"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
}

#[tokio::test]
async fn test_get_skimmed_active_entry_gateways() {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/entry-gateways/active",
        base_url()
    );
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    assert!(
        res.status().is_success(),
        "Expected 2xx but got {}",
        res.status()
    );

    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .expect("Expected a value for 'current_epoch_id'");

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be an array"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
}

#[tokio::test]
async fn test_get_skimmed_all_entry_gateways() {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/entry-gateways/all",
        base_url()
    );
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    assert!(
        res.status().is_success(),
        "Expected 2xx but got {}",
        res.status()
    );

    let json: Value = res
        .json()
        .await
        .unwrap_or_else(|err| panic!("Failed to parse response as JSON: {}", err));
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .expect("Expected a value for 'current_epoch_id'");

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be an array"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
}

// Add the remining tests as the endpoints become active
