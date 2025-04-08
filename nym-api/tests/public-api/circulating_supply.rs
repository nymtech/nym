mod utils;
use utils::{base_url, test_client, validate_json_response};

#[tokio::test]
async fn test_get_circulating_supply() {
    let url = format!("{}/v1/circulating-supply", base_url());
    let res = test_client().get(&url).send().await.unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;

    assert!(
        json.get("circulating_supply").is_some(),
        "Expected a value for 'circulating_supply'"
    );
}

#[tokio::test]
async fn test_get_circulating_supply_value() {
    let url = format!(
        "{}/v1/circulating-supply/circulating-supply-value",
        base_url()
    );
    let res = test_client().get(&url).send().await.unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;

    assert!(
        json.is_number(),
        "Expected a number for the circulating supply value"
    );
    let number = json.as_f64().unwrap();
    assert!(number >= 0.0, "Circulating supply should not be negative");
}

#[tokio::test]
async fn test_get_total_supply_value() {
    let url = format!("{}/v1/circulating-supply/total-supply-value", base_url());
    let res = test_client().get(&url).send().await.unwrap_or_else(|err| panic!("Failed to send request to {}: {}", url, err));
    let json = validate_json_response(res).await;

    assert!(json.is_number(), "Expected a number for total supply value");
    let number = json.as_f64().unwrap();
    assert!(number >= 0.0, "Total supply should not be negative");
}
