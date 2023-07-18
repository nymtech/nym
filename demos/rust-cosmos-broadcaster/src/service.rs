use crate::DEFAULT_VALIDATOR_RPC;
use bs58;
use cosmrs::rpc::{Client, HttpClient};
use cosmrs::{tendermint, AccountId};
use nym_validator_client::nyxd::CosmWasmClient;

pub async fn create_broadcaster() -> HttpClient {
    let broadcaster: HttpClient = HttpClient::new(DEFAULT_VALIDATOR_RPC).unwrap();
    broadcaster
}

pub async fn get_sequence(
    broadcaster: HttpClient,
    signer_address: AccountId,
) -> Result<crate::SequenceRequestResponse, std::io::Error> {
    // get signer information
    let sequence = broadcaster.get_sequence(&signer_address).await.unwrap();
    let chain_id: tendermint::chain::Id = broadcaster.get_chain_id().await.unwrap();
    Ok(crate::SequenceRequestResponse {
        account_number: sequence.account_number,
        sequence: sequence.sequence,
        chain_id,
    })
}

pub async fn broadcast(
    base58_tx_bytes: String,
    broadcaster: HttpClient,
) -> Result<crate::BroadcastResponse, std::io::Error> {
    // decode the base58 tx to vec<u8>
    let tx_bytes = bs58::decode(base58_tx_bytes).into_vec().unwrap();

    // this is our sender address hardcoded for ease of the demo logging
    let from_address: AccountId = "n1p8ayfmdash352gh6yy8zlxk24dm6yzc9mdq0p6".parse().unwrap();

    // compare balances from before and after the tx
    let before = broadcaster
        .get_balance(&from_address, "unym".to_string())
        .await
        .unwrap()
        .unwrap();

    // broadcast the tx
    println!("broadcasting the tx to Nyx blockchain");
    let broadcast_res = Client::broadcast_tx_commit(&broadcaster, tx_bytes.into())
        .await
        .unwrap();

    let after = broadcaster
        .get_balance(&from_address, "unym".to_string())
        .await
        .unwrap()
        .unwrap();

    println!(
        "returned transaction hash: {:#?}",
        broadcast_res.hash.to_string()
    );
    println!("balance before transaction: {before}");
    println!("balance after transaction:  {after}");
    println!("returning tx hash to sender");

    let success: bool = after.amount < before.amount;

    Ok(crate::BroadcastResponse {
        tx_hash: serde_json::to_string(&broadcast_res.hash).unwrap(),
        success,
    })
}
