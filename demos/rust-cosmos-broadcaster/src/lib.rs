use serde::{Deserialize, Serialize}; 
use cosmrs::{AccountId, tendermint};
use std::path::PathBuf;
use nym_sdk::mixnet::{StoragePaths, MixnetClientBuilder, MixnetClient};
pub mod client; 
pub mod service;

pub const DEFAULT_VALIDATOR_RPC: &str = "https://qwerty-validator.qa.nymte.ch"; 
pub const DEFAULT_DENOM: &str = "unym"; 
pub const DEFAULT_PREFIX: &str = "n"; 
// pub const DEFAULT_SERVICE_NYM_ADDRESS: &str = "HfbesQm2pRYCN4BAdYXhkqXBbV1Pp929mtKsESVeWXh8.8AgoUPUQbXNBCPaqAaWd3vnxhc9484qwfgrrQwBngQk2@Ck8zpXTSXMtS9YZ7k7a5BiaoLZfffWuqGWLndujh4Lw4"; 

#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceRequest {
    pub validator: String, 
    pub signer_address: AccountId, 
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceRequestResponse {
    pub account_number: u64,
    pub sequence: u64, 
    pub chain_id: tendermint::chain::Id
}
#[derive(Deserialize, Serialize, Debug)]
pub struct BroadcastRequest {
    pub base58_tx_bytes: String
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BroadcastResponse{
    pub tx_hash: String
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RequestTypes {
    Sequence(SequenceRequest),
    Broadcast(BroadcastRequest)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ResponseTypes {
    Sequence(SequenceRequestResponse), 
    Broadcast(BroadcastResponse)
}

pub async fn create_client(config_path: PathBuf) -> MixnetClient {
    let config_dir = config_path;
    let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();
    let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .await
        .unwrap();
    
    client.connect_to_mixnet().await.unwrap()
}