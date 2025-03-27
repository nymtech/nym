mod utils;
use utils::{base_url, test_client, validate_json_response};
use tokio;

#[tokio::test]
async fn test_get_current_epoch() {
    let url = format!("{}/v1/epoch/current", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;

    assert!(json.get("id").is_some(), "Missing 'id'");
    assert!(json.get("current_epoch_start").is_some(), "Missing 'current_epoch_start'");
    assert!(json.get("total_elapsed_epochs").is_some(), "Missing 'total_elapsed_epochs'");
}

#[tokio::test]
async fn test_get_reward_params() {
    let url = format!("{}/v1/epoch/reward_params", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    let json = validate_json_response(res).await;

    let interval = json.get("interval").expect("Missing 'interval' field");
    assert!(interval.get("reward_pool").is_some(), "Missing 'interval.reward_pool'");

    let rewarded_set = json.get("rewarded_set").expect("Missing 'rewarded_set' field");
    assert!(rewarded_set.get("exit_gateways").is_some(), "Missing 'rewarded_set.exit_gateways'");
}
