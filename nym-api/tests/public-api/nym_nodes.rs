mod utils;
use utils::{base_url, test_client, get_any_node_id, validate_json_response};
use tokio;
use chrono::Utc;

#[tokio::test]
async fn test_get_bonded_nodes() {
    let url = format!("{}/v1/nym-nodes/bonded", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;
    let data = json.get("data").expect("Missing 'data' field");

    assert!(data.is_array(), "Expected 'data' to be an array");
    assert!(
        data.as_array().unwrap().len() > 0,
        "Expected at least one bonded node"
    );
}

#[tokio::test]
async fn test_get_described_nodes() {
    let url = format!("{}/v1/nym-nodes/described", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;
    let data = json.get("data").expect("Missing 'data' field");

    assert!(data.is_array(), "Expected 'data' to be an array");
    assert!(
        data.as_array().unwrap().len() > 0,
        "Expected at least one node to appear"
    );
}

// TODO enable this once noise is properly integrated
// #[tokio::test]
// async fn test_get_noise() {
//     let url = format!("{}/v1/nym-nodes/noise", base_url());
//     let res = test_client().get(&url).send().await.unwrap();
//     let json = validate_json_response(res).await;
// }

#[tokio::test]
async fn test_get_rewarded_set() {
    let url = format!("{}/v1/nym-nodes/rewarded-set", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;
    let exit_gateways = json.get("exit_gateways").expect("Missing 'exit_gateways' field");

    assert!(exit_gateways.is_array(), "Expected 'exit_gateways' to be an array");
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
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;
    let annotation = json.get("annotation").expect("Missing 'annotation' field");

    assert!(annotation.get("last_24h_performance").is_some(), "Missing 'last_24h_performance'");
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
    assert!(json.get("performance").is_some(), "Missing 'performance' field");
}

#[tokio::test]
async fn test_get_performance_history() {
    let id = get_any_node_id().await;
    let url = format!("{}/v1/nym-nodes/performance-history/{}", base_url(), id);
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;
    let data = json
        .get("history")
        .and_then(|h| h.get("data"))
        .expect("Missing 'history.data' field");

    assert!(data.is_array(), "Expected 'history.data' to be an array");
    assert!(
        data.as_array().unwrap().len() > 0,
        "Expected at least one performance history entry"
    );
}

#[tokio::test]
async fn test_get_performance() {
    let id = get_any_node_id().await;
    let url = format!("{}/v1/nym-nodes/performance/{}", base_url(), id);
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;
    assert!(json.get("node_id").is_some(), "Missing 'node_id'");
    assert!(json.get("performance").is_some(), "Missing 'performance'");
}

#[tokio::test]
async fn test_get_uptime_history() {
    let id = get_any_node_id().await;
    let url = format!("{}/v1/nym-nodes/uptime-history/{}", base_url(), id);
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;
    let data = json
        .get("history")
        .and_then(|h| h.get("data"))
        .expect("Missing 'history.data' field");

    assert!(data.is_array(), "Expected 'history.data' to be an array");
    assert!(
        data.as_array().unwrap().len() > 0,
        "Expected at least one performance history entry"
    );
}

// TODO add the POST request test for `refresh-described` 