use nym_validator_client::nyxd::CosmWasmClient;
use cosmrs::rpc::{HttpClient, Client};
use cosmrs::{AccountId, tendermint};
use bs58; 

pub async fn get_sequence(validator: String, signer_address: AccountId) -> crate::SequenceRequestResponse {
    /*
      TODO create broadcaster in different fn and build on setup - pass to both fns as arg 
     */
    let broadcaster = HttpClient::new(validator.as_str()).unwrap();
    // get signer information
    let sequence = broadcaster.get_sequence(&signer_address).await.unwrap();
    let chain_id: tendermint::chain::Id = broadcaster.get_chain_id().await.unwrap();
    let res = crate::SequenceRequestResponse { account_number: sequence.account_number, sequence: sequence.sequence, chain_id };
    res  
}

pub async fn broadcast(base58_tx_bytes: String) -> crate::BroadcastResponse {
    // decode the base58 tx to vec<u8>
    let tx_bytes = bs58::decode(base58_tx_bytes).into_vec().unwrap();  
    println!("decoded tx bytes: {:#?}", tx_bytes); 
    
    /*
      TODO create broadcaster in different fn and build on setup - pass to both fns as arg 
     */
    let broadcaster = HttpClient::new("https://qwerty-validator.qa.nymte.ch").unwrap();
    
    let to_address: AccountId = "n1p8ayfmdash352gh6yy8zlxk24dm6yzc9mdq0p6".parse().unwrap();

    // compare balances from before and after the tx
    let before = broadcaster
        .get_balance(&to_address, "unym".to_string())
        .await
        .unwrap()
        .unwrap();

    // broadcast the tx
    let broadcast_res = Client::broadcast_tx_commit(&broadcaster, tx_bytes.into())
        .await
        .unwrap();

    let after = broadcaster
        .get_balance(&to_address, "unym".to_string())
        .await
        .unwrap()
        .unwrap();
 
    println!("{:#?}", broadcast_res.hash); 
    println!("balance before: {before}");
    println!("balance after:  {after}");

    let res = crate::BroadcastResponse {
      tx_hash: serde_json::to_string(&broadcast_res.hash).unwrap()
    }; 
    res  
}