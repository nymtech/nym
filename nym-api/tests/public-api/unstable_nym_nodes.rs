use crate::utils::{base_url, make_request, validate_json_response};

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_skimmed_nodes_active() -> Result<(), String> {
    let url = format!("{}/v1/unstable/nym-nodes/skimmed/active", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let data = json
        .get("nodes")
        .and_then(|r| r.get("data"))
        .ok_or("Missing 'data' field")?;

    assert!(data.is_array(), "Expected 'data' to be an array");
    assert!(
        !data.as_array().unwrap().is_empty(),
        "Expected at least one node to appear"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_skimmed_active_mixnodes() -> Result<(), String> {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/mixnodes/active",
        base_url()?
    );
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .ok_or("Missing 'current_epoch_id'")?;

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be a number"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_skimmed_all_mixnodes() -> Result<(), String> {
    let url = format!("{}/v1/unstable/nym-nodes/skimmed/mixnodes/all", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .ok_or("Missing 'current_epoch_id'")?;

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be a number"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_skimmed_active_exit_gateways() -> Result<(), String> {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/exit-gateways/active",
        base_url()?
    );
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .ok_or("Missing 'current_epoch_id'")?;

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be a number"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_skimmed_all_exit_gateways() -> Result<(), String> {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/exit-gateways/all",
        base_url()?
    );
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .ok_or("Missing 'current_epoch_id'")?;

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be a number"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_skimmed_active_entry_gateways() -> Result<(), String> {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/entry-gateways/active",
        base_url()?
    );
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .ok_or("Missing 'current_epoch_id'")?;

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be a number"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_skimmed_all_entry_gateways() -> Result<(), String> {
    let url = format!(
        "{}/v1/unstable/nym-nodes/skimmed/entry-gateways/all",
        base_url()?
    );
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let current_epoch_id = json
        .get("status")
        .and_then(|r| r.get("fresh"))
        .and_then(|r| r.get("current_epoch_id"))
        .ok_or("Missing 'current_epoch_id'")?;

    assert!(
        current_epoch_id.is_number(),
        "Expected 'current_epoch_id' to be a number"
    );
    assert!(
        json.get("refreshed_at").is_some(),
        "Expected a value for 'refreshed_at'"
    );
    Ok(())
}

// Add the remining tests as the endpoints become active
