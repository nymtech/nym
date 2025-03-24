mod utils;
use utils::{base_url, test_client};
use serde_json::Value;
use tokio;

#[tokio::test]
async fn test_get_circulating_supply() {
    let url = format!("{}/v1/circulating-supply", base_url());
    let res = test_client().get(&url).send().await.unwrap();

    assert!(res.status().is_success(), "Expected 200 OK, got {}", res.status());
    let json: Value = res.json().await.expect("Invalid JSON response");

    assert!(json.get("circulating_supply").is_some(), "Missing 'circulating_supply' field");
}

#[tokio::test]
async fn test_get_circulating_supply_value() {
    let url = format!("{}/v1/circulating-supply/circulating-supply-value", base_url());
    let res = test_client().get(&url).send().await.unwrap();

    assert!(res.status().is_success(), "Expected 200 OK, got {}", res.status());
    let json: Value = res.json().await.expect("Invalid JSON");

    assert!(json.is_number(), "Expected a number for the circulating supply value");
    let number = json.as_f64().unwrap();
    assert!(number >= 0.0, "Circulating supply should be non-negative");
}

#[tokio::test]
async fn test_get_total_supply_value() {
    let url = format!("{}/v1/circulating-supply/total-supply-value", base_url());
    let res = test_client().get(&url).send().await.unwrap();

    assert!(res.status().is_success(), "Expected 200 OK, got {}", res.status());
    let json: Value = res.json().await.expect("Invalid JSON");

    assert!(json.is_number(), "Expected a number for total supply value");
    let number = json.as_f64().unwrap();
    assert!(number >= 0.0, "Total supply should be non-negative");
}
