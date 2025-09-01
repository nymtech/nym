use crate::utils::{base_url, get_any_node_id, make_request, test_client, validate_json_response};
use time::OffsetDateTime;

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_bonded_nodes() -> Result<(), String> {
    let url = format!("{}/v1/nym-nodes/bonded", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let data = json
        .get("data")
        .ok_or("Expected a value for 'data' field")?;

    assert!(data.is_array(), "Expected 'data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one bonded node"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_described_nodes() -> Result<(), String> {
    let url = format!("{}/v1/nym-nodes/described", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let data = json
        .get("data")
        .ok_or("Expected a value for 'data' field")?;

    assert!(data.is_array(), "Expected 'data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one node to appear"
    );
    Ok(())
}

// TODO enable this once noise is properly integrated
// #[tokio::test]
#[test_with::env(NYM_API)]
// async fn test_get_noise() -> Result<(), String> {
//     let url = format!("{}/v1/nym-nodes/noise", base_url()?);
//     let res = test_client().get(&url).send().await.map_err(|err| panic!("Failed to send request to {}: {}", url, err))?;
//     let json = validate_json_response(res).await;
// }
#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_rewarded_set() -> Result<(), String> {
    let url = format!("{}/v1/nym-nodes/rewarded-set", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let exit_gateways = json
        .get("exit_gateways")
        .ok_or("Expected a value for 'exit_gateways' field")?;

    assert!(
        exit_gateways.is_array(),
        "Expected 'exit_gateways' to be an array"
    );
    assert!(
        !exit_gateways.as_array().unwrap().is_empty(),
        "We have no exit gateways!!"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_annotation_for_node() -> Result<(), String> {
    let id = get_any_node_id().await?;
    let url = format!("{}/v1/nym-nodes/annotation/{}", base_url()?, id);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let annotation = json
        .get("annotation")
        .ok_or("Expected a value for 'annotation' field")?;

    assert!(
        annotation.get("last_24h_performance").is_some(),
        "Expected a value for 'last_24h_performance'"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_historical_performance() -> Result<(), String> {
    let id = get_any_node_id().await?;
    let date = OffsetDateTime::now_utc().date().to_string();
    let url = format!("{}/v1/nym-nodes/historical-performance/{}", base_url()?, id);
    let res = test_client()
        .get(&url)
        .query(&[("date", date)])
        .send()
        .await
        .map_err(|err| format!("Failed to send request to {url}: {err}"))?;
    let json = validate_json_response(res).await?;

    assert!(
        json.get("performance").is_some(),
        "Expected a value for 'performance' field"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_performance_history() -> Result<(), String> {
    let id = get_any_node_id().await?;
    let url = format!("{}/v1/nym-nodes/performance-history/{}", base_url()?, id);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let data = json
        .get("history")
        .and_then(|h| h.get("data"))
        .ok_or("Expected a value for 'history.data'")?;

    assert!(data.is_array(), "Expected 'history.data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one performance history entry"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_performance() -> Result<(), String> {
    let id = get_any_node_id().await?;
    let url = format!("{}/v1/nym-nodes/performance/{}", base_url()?, id);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;

    assert!(
        json.get("node_id").is_some(),
        "Expected a value for 'node_id'"
    );
    assert!(
        json.get("performance").is_some(),
        "Expected a value for 'performance'"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_uptime_history() -> Result<(), String> {
    let id = get_any_node_id().await?;
    let url = format!("{}/v1/nym-nodes/uptime-history/{}", base_url()?, id);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let data = json
        .get("history")
        .and_then(|h| h.get("data"))
        .ok_or("Expected a value for 'history.data'")?;

    assert!(data.is_array(), "Expected 'history.data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one performance history entry"
    );
    Ok(())
}

// TODO add the POST request test for `refresh-described`
