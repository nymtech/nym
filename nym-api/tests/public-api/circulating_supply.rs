use crate::utils::{base_url, make_request, validate_json_response};

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_circulating_supply() -> Result<(), String> {
    let url = format!("{}/v1/circulating-supply", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;

    assert!(
        json.get("circulating_supply").is_some(),
        "Expected a value for 'circulating_supply'"
    );
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_circulating_supply_value() -> Result<(), String> {
    let url = format!(
        "{}/v1/circulating-supply/circulating-supply-value",
        base_url()?
    );
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;

    assert!(
        json.is_number(),
        "Expected a number for the circulating supply value"
    );
    let number = json.as_f64().unwrap_or(-1.0);
    assert!(number >= 0.0, "Circulating supply should not be negative");
    Ok(())
}

#[tokio::test]
#[test_with::env(NYM_API)]
async fn test_get_total_supply_value() -> Result<(), String> {
    let url = format!("{}/v1/circulating-supply/total-supply-value", base_url()?);
    let res = make_request(&url).await?;
    let json = validate_json_response(res).await?;

    assert!(json.is_number(), "Expected a number for total supply value");
    let number = json.as_f64().unwrap_or(-1.0);
    assert!(number >= 0.0, "Total supply shouldn't be negative");
    Ok(())
}
