use crate::utils::{base_url, make_request, validate_json_response};

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_current_epoch() -> Result<(), String> {
    let url = format!("{}/v1/epoch/current", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;

    assert!(json.get("id").is_some(), "Expected a value for 'id'");
    assert!(
        json.get("current_epoch_start").is_some(),
        "Expected a value for `current_epoch_start`"
    );
    assert!(
        json.get("total_elapsed_epochs").is_some(),
        "Expected a value for `total_elapsed_epochs`"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_reward_params() -> Result<(), String> {
    let url = format!("{}/v1/epoch/reward_params", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;
    let interval = json
        .get("interval")
        .ok_or("Expected a value for 'interval'")?;
    assert!(
        interval.get("reward_pool").is_some(),
        "Expected a value for 'interval.reward_pool'"
    );

    let rewarded_set = json
        .get("rewarded_set")
        .ok_or("Expected a value for 'rewarded_set'")?;
    assert!(
        rewarded_set.get("exit_gateways").is_some(),
        "Expected a value for 'rewarded_set.exit_gateways'"
    );
    Ok(())
}
