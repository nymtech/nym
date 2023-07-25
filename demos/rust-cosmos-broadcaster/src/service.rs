use crate::DEFAULT_VALIDATOR_RPC;
use bs58;
use cosmrs::rpc::{Client, HttpClient};
use cosmrs::{tendermint, AccountId};
use nym_validator_client::nyxd::{error::NyxdError, CosmWasmClient};

pub async fn create_broadcaster() -> anyhow::Result<HttpClient> {
    let broadcaster: HttpClient = HttpClient::new(DEFAULT_VALIDATOR_RPC)?;
    Ok(broadcaster)
}

pub async fn get_sequence(
    broadcaster: HttpClient,
    signer_address: AccountId,
) -> Result<crate::SequenceRequestResponse, NyxdError> {
    // get signer information
    let sequence = broadcaster.get_sequence(&signer_address).await?;
    let chain_id: tendermint::chain::Id = broadcaster.get_chain_id().await?;
    Ok(crate::SequenceRequestResponse {
        account_number: sequence.account_number,
        sequence: sequence.sequence,
        chain_id,
    })
}

pub async fn broadcast(
    base58_tx_bytes: String,
    broadcaster: HttpClient,
) -> anyhow::Result<crate::BroadcastResponse> {
    // decode the base58 tx to vec<u8>
    let tx_bytes = bs58::decode(base58_tx_bytes).into_vec()?;

    // this is our sender address hardcoded for ease of the demo logging
    let from_address: AccountId = "n19wln95zj5r3wnepgk6nf7lqx0zgufvgtlvyawf".parse().unwrap();

    // compare balances from before and after the tx
    let before = broadcaster
        .get_balance(&from_address, "unym".to_string())
        .await
        .unwrap()
        .unwrap();

    // broadcast the tx
    println!("broadcasting tx to validator");
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

    let success: bool = broadcast_res.deliver_tx.code.is_ok();

    Ok(crate::BroadcastResponse {
        tx_hash: serde_json::to_string(&broadcast_res.hash).unwrap(),
        success,
    })
}
