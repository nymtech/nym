use crate::utils::{base_url, make_request, validate_json_response};

#[tokio::test]
async fn test_health() -> Result<(), String> {
    let url = format!("{}/v1/api-status/health", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;

    assert_eq!(json["status"], "up", "Expected status is 'up'");
    Ok(())
}

#[tokio::test]
async fn test_build_information() -> Result<(), String> {
    let url = format!("{}/v1/api-status/build-information", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;

    assert!(
        json.get("build_version").is_some(),
        "Expected a value for 'build_version'"
    );
    Ok(())
}

// ECASH API Test
/*
#[tokio::test]
async fn test_signer_information() -> Result<(), String> {
    let url = format!("{}/v1/api-status/signer-information", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;

    assert!(
        json.get("signer_address").is_some(),
        "Expected a value for 'signer_address'"
    );
    Ok(())
}
*/
