// tests/api_status.rs
use serde_json::Value;
use tokio;
mod utils;
use utils::{base_url, test_client};

#[tokio::test]
async fn test_health() {
    let url = format!("{}/v1/api-status/health", base_url());
    let res = test_client().get(&url).send().await.unwrap();

    assert!(res.status().is_success(), "Expected 2xx but got {}", res.status());
    let json: Value = res.json().await.expect("Invalid JSON");

    assert_eq!(json["status"], "up", "Expected status to be 'up'");
}

// #[tokio::test]
// async fn test_signer_information() {
//     let url = format!("{}/v1/api-status/signer-information", base_url());
//     let res = test_client().get(&url).send().await.unwrap();

//     assert!(res.status().is_success());
//     let json: Value = res.json().await.expect("Invalid JSON");

//     assert!(json.get("signer_address").is_some(), "Missing 'signer_address'");
//     // TODO add an OR for "this api does not expose zk-nym signing functionalities" when checkign the main api 
// }

#[tokio::test]
async fn test_build_information() {
    let url = format!("{}/v1/api-status/build-information", base_url());
    let res = test_client().get(&url).send().await.unwrap();

    assert!(res.status().is_success());
    let json: Value = res.json().await.expect("Invalid JSON");

    assert!(json.get("build_version").is_some(), "Missing 'build_version'");
}
