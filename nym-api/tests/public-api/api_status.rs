use tokio;
mod utils;
use utils::{base_url, test_client, validate_json_response};

#[tokio::test]
async fn test_health() {
    let url = format!("{}/v1/api-status/health", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;

    assert_eq!(json["status"], "up", "Expected status is 'up'");
}

#[tokio::test]
async fn test_build_information() {
    let url = format!("{}/v1/api-status/build-information", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;

    assert!(json.get("build_version").is_some(), "Missing 'build_version'");
}

// ECASH API TEST 
// #[tokio::test]
// async fn test_signer_information() {
//     let url = format!("{}/v1/api-status/signer-information", base_url());
//     println!("{}", url);
//     let res = test_client().get(&url).send().await.unwrap();

//     assert!(res.status().is_success(), "Expected 2xx but got {}", res.status());
//     let json: Value = res.json().await.unwrap_or_else(|err| panic!("Invalid JSON response: {}", err));

//     assert!(json.get("signer_address").is_some(), "Missing 'signer_address'");
// }