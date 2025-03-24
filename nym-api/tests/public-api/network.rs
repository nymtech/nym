mod utils;
use utils::{base_url, test_client};
use serde_json::Value;
use tokio;

#[tokio::test]
async fn test_get_chain_status() {
    let url = format!("{}/v1/network/chain-status", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    
    assert!(res.status().is_success(), "Expected 200 OK, got {}", res.status());
    let json: Value = res.json().await.expect("Invalid JSON");

    let block_header = json
        .get("status")
        .and_then(|s| s.get("latest_block"))
        .and_then(|b| b.get("block"))
        .and_then(|b| b.get("header"))
        .expect("Missing 'status.block.header'");

    assert!(block_header.get("chain_id").is_some(), "Missing 'chain_id'");
    assert!(block_header.get("height").is_some(), "Missing 'height'");
    
}

#[tokio::test]
async fn test_get_network_details() {
    let url = format!("{}/v1/network/details", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    
    assert!(res.status().is_success(), "Expected 200 OK, got {}", res.status());
    let json: Value = res.json().await.expect("Invalid JSON");

    assert!(json.get("connected_nyxd").is_some(), "Missing 'connected_nyxd'");
    let contracts = json
    .get("network")
    .and_then(|s| s.get("contracts"))
    .expect("Missing 'contracts'");
    assert!(contracts.get("mixnet_contract_address").is_some(), "Missing 'mixnet_contract_address'");
}

#[tokio::test]
async fn test_get_nym_contracts() {
    let url = format!("{}/v1/network/nym-contracts", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    
    assert!(res.status().is_success(), "Expected 200 OK, got {}", res.status());
    let json: Value = res.json().await.expect("Invalid JSON");

    assert!(json.get("nym-mixnet-contract").is_some(), "Missing 'nym-mixnet-contract'");
    assert!(json.get("nym-ecash-contract").is_some(), "Missing 'nym-ecash-contract'");
}

#[tokio::test]
async fn test_get_nym_contracts_detailed() {
    let url = format!("{}/v1/network/nym-contracts-detailed", base_url());
    let res = test_client().get(&url).send().await.unwrap();
    
    assert!(res.status().is_success(), "Expected 200 OK, got {}", res.status());
    let json: Value = res.json().await.expect("Invalid JSON");

    let mixnet_contract = json
    .get("nym-mixnet-contract")
    .and_then(|s| s.get("details"))
    .expect("Missing details for mixnet contract");
    assert!(mixnet_contract.get("commit_branch").is_some(), "Missing 'commit_branch'");

    let mixnet_contract = json
    .get("nym-ecash-contract")
    .and_then(|s| s.get("details"))
    .expect("Missing details for ecash contract");
    assert!(mixnet_contract.get("commit_branch").is_some(), "Missing 'commit_branch'");
}
