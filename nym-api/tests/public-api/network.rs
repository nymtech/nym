use crate::utils::{base_url, test_client, validate_json_response};

#[tokio::test]
async fn test_get_chain_status() -> Result<(), String> {
    let url = format!("{}/v1/network/chain-status", base_url()?);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("Failed to send request to {url}: {err}"))?;
    let json = validate_json_response(res).await?;

    let block_header = json
        .get("status")
        .and_then(|s| s.get("latest_block"))
        .and_then(|b| b.get("block"))
        .and_then(|b| b.get("header"))
        .ok_or("Expected a value for 'status.block.header'")?;

    assert!(
        block_header.get("chain_id").is_some(),
        "Expected a value for 'chain_id'"
    );
    assert!(
        block_header.get("height").is_some(),
        "Expected a value for 'height'"
    );
    Ok(())
}

#[tokio::test]
async fn test_get_network_details() -> Result<(), String> {
    let url = format!("{}/v1/network/details", base_url()?);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("Failed to send request to {url}: {err}"))?;
    let json = validate_json_response(res).await?;

    assert!(
        json.get("connected_nyxd").is_some(),
        "Expected a value for 'connected_nyxd'"
    );
    let contracts = json
        .get("network")
        .and_then(|s| s.get("contracts"))
        .ok_or("Expected a value for 'contracts'")?;
    assert!(
        contracts.get("mixnet_contract_address").is_some(),
        "Expected a value for 'mixnet_contract_address'"
    );
    Ok(())
}

#[tokio::test]
async fn test_get_nym_contracts() -> Result<(), String> {
    let url = format!("{}/v1/network/nym-contracts", base_url()?);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("Failed to send request to {url}: {err}"))?;
    let json = validate_json_response(res).await?;

    assert!(
        json.get("nym-mixnet-contract").is_some(),
        "Expected a value for 'nym-mixnet-contract'"
    );
    assert!(
        json.get("nym-ecash-contract").is_some(),
        "Expected a value for 'nym-ecash-contract'"
    );
    Ok(())
}

#[tokio::test]
async fn test_get_nym_contracts_detailed() -> Result<(), String> {
    let url = format!("{}/v1/network/nym-contracts-detailed", base_url()?);
    let res = test_client()
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("Failed to send request to {url}: {err}"))?;
    let json = validate_json_response(res).await?;

    let mixnet_contract = json
        .get("nym-mixnet-contract")
        .and_then(|s| s.get("details"))
        .ok_or("Expected details for the mixnet contract")?;
    assert!(
        mixnet_contract.get("commit_branch").is_some(),
        "Expected a value for 'commit_branch'"
    );

    let ecash_contract = json
        .get("nym-ecash-contract")
        .and_then(|s| s.get("details"))
        .ok_or("Expected details for the ecash contract")?;
    assert!(
        ecash_contract.get("commit_branch").is_some(),
        "Expected a value for 'commit_branch'"
    );
    Ok(())
}
