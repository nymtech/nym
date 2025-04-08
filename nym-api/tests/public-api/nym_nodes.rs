mod utils;
use crate::utils::{base_url, get_any_node_id, test_client, validate_json_response};
use chrono::Utc;

#[tokio::test]
async fn test_get_bonded_nodes() {
    let url = format!("{}/v1/nym-nodes/bonded", base_url());
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;
    let data = json.get("data").expect("Expected a value for 'data' field");

    assert!(data.is_array(), "Expected 'data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one bonded node"
    );
}

#[tokio::test]
async fn test_get_described_nodes() {
    let url = format!("{}/v1/nym-nodes/described", base_url());
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;
    let data = json.get("data").expect("Expected a value for 'data' field");

    assert!(data.is_array(), "Expected 'data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one node to appear"
    );
}

// TODO enable this once noise is properly integrated
// #[tokio::test]
// async fn test_get_noise() {
//     let url = format!("{}/v1/nym-nodes/noise", base_url());
//     let res = test_client().get(&url).send().await.unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
//     let json = validate_json_response(res).await;
// }

#[tokio::test]
async fn test_get_rewarded_set() {
    let url = format!("{}/v1/nym-nodes/rewarded-set", base_url());
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;
    let exit_gateways = json
        .get("exit_gateways")
        .expect("Expected a value for 'exit_gateways' field");

    assert!(
        exit_gateways.is_array(),
        "Expected 'exit_gateways' to be an array"
    );
    assert!(
        exit_gateways.as_array().unwrap().len() > 0,
        "We have no exit gateways!!"
    );
}

#[tokio::test]
async fn test_get_annotation_for_node() {
    let id = get_any_node_id().await;
    println!("Using node_id: {}", id);
    let url = format!("{}/v1/nym-nodes/annotation/{}", base_url(), id);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;
    let annotation = json
        .get("annotation")
        .expect("Expected a value for 'annotation' field");

    assert!(
        annotation.get("last_24h_performance").is_some(),
        "Expected a value for 'last_24h_performance'"
    );
}
#[tokio::test]
async fn test_get_historical_performance() {
    let id = get_any_node_id().await;
    let date = Utc::now().date_naive().to_string();

    let url = format!("{}/v1/nym-nodes/historical-performance/{}", base_url(), id);
    let res = test_client()
        .get(&url)
        .query(&[("date", date)])
        .send()
        .await
        .unwrap();

    let json = validate_json_response(res).await;
    assert!(
        json.get("performance").is_some(),
        "Expected a value for 'performance' field"
    );
}

#[tokio::test]
async fn test_get_performance_history() {
    let id = get_any_node_id().await;
    let url = format!("{}/v1/nym-nodes/performance-history/{}", base_url(), id);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;
    let data = json
        .get("history")
        .and_then(|h| h.get("data"))
        .expect("Expected a value for 'history.data'");

    assert!(data.is_array(), "Expected 'history.data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one performance history entry"
    );
}

#[tokio::test]
async fn test_get_performance() {
    let id = get_any_node_id().await;
    let url = format!("{}/v1/nym-nodes/performance/{}", base_url(), id);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;
    assert!(
        json.get("node_id").is_some(),
        "Expected a value for 'node_id'"
    );
    assert!(
        json.get("performance").is_some(),
        "Expected a value for 'performance'"
    );
}

#[tokio::test]
async fn test_get_uptime_history() {
    let id = get_any_node_id().await;
    let url = format!("{}/v1/nym-nodes/uptime-history/{}", base_url(), id);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;
    let data = json
        .get("history")
        .and_then(|h| h.get("data"))
        .expect("Expected a value for 'history.data'");

    assert!(data.is_array(), "Expected 'history.data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one performance history entry"
    );
}

// TODO add the POST request test for `refresh-described`
