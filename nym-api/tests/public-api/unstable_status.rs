use crate::utils::{base_url, get_gateway_identity_key, get_mixnode_node_id, test_client};
use serde_json::Value;

#[tokio::test]
async fn test_get_gateway_unstable_test_results() {
    let identity = get_gateway_identity_key().await;
    let url = format!(
        "{}/v1/status/gateways/unstable/{}/test-results",
        base_url(),
        identity
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
        .unwrap_or_else(|err| panic!("Invalid JSON response: {}", err));
    let data_array = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing or invalid 'data' array");

    assert!(!data_array.is_empty(), "'data' array is empty");

    let gateway = data_array[0]
        .get("test_routes")
        .and_then(|r| r.get("gateway"))
        .expect("Expected a value for 'test_routes.gateway'");

    assert!(
        gateway.get("node_id").is_some(),
        "Expected a value for 'node_id' in gateway"
    );
    assert!(
        gateway.get("identity_key").is_some(),
        "Expected a value for 'identity_key' in gateway"
    );
}

#[tokio::test]
async fn test_get_mixnode_unstable_test_results() {
    let mix_id = get_mixnode_node_id().await;
    let url = format!(
        "{}/v1/status/mixnodes/unstable/{}/test-results",
        base_url(),
        mix_id
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
        .unwrap_or_else(|err| panic!("Invalid JSON response: {}", err));
    let data_array = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing or invalid 'data' array");

    assert!(!data_array.is_empty(), "'data' array is empty");

    let layer3 = data_array[0]
        .get("test_routes")
        .and_then(|r| r.get("layer3"))
        .expect("Expected a value for 'test_routes.layer3'");

    assert!(
        layer3.get("node_id").is_some(),
        "Expected a value for 'node_id' in layer3"
    );
    assert!(
        layer3.get("identity_key").is_some(),
        "Expected a value for 'identity_key' in layer3"
    );
}

#[tokio::test]
async fn test_get_latest_network_monitor_run_details() {
    let url = format!(
        "{}/v1/status/network-monitor/unstable/run/latest/details",
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
    let monitor_run_id = json
        .get("monitor_run_id")
        .and_then(|v| v.as_number())
        .expect("Missing or invalid 'monitor_run_id'");

    let follow_up_url = format!(
        "{}/v1/status/network-monitor/unstable/run/{}/details",
        base_url(),
        monitor_run_id
    );
    let follow_up_res = test_client().get(&follow_up_url).send().await.unwrap();
    assert!(follow_up_res.status().is_success());
}
