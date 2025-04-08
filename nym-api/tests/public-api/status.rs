mod utils;
use serde_json::Value;
use tokio;
use utils::{base_url, test_client};

#[tokio::test]
async fn test_get_config_score_details() {
    let url = format!("{}/v1/status/config-score-details", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    assert!(
        res.status().is_success(),
        "Expected 200 OK, got {}",
        res.status()
    );

    let json: Value = res.json().await.unwrap();

    let version_history = json
        .get("version_history")
        .and_then(|v| v.as_array())
        .expect("Missing or invalid 'version_history' array");

    assert!(
        !version_history.is_empty(),
        "'version_history' should not be empty"
    );

    let max_entry = version_history
        .iter()
        .max_by_key(|entry| entry.get("id").and_then(|id| id.as_u64()).unwrap_or(0))
        .expect("Unable to find max id entry");

    let semver = max_entry
        .get("version_information")
        .and_then(|v| v.get("semver"))
        .and_then(|v| v.as_str());

    assert!(
        semver.is_some(),
        "Expected a value for 'semver' in the highest id entry"
    );
}

// TODO add the POST request tests for:
// submit-gateway-monitoring-results
// submit-node-monitoring-results
