use cosmrs::{tendermint, AccountId};
use nym_sdk::mixnet::{MixnetClient, MixnetClientBuilder, StoragePaths, ReconstructedMessage, AnonymousSenderTag};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
pub mod client;
pub mod service;

pub const DEFAULT_VALIDATOR_RPC: &str = "https://qwerty-validator.qa.nymte.ch";
pub const DEFAULT_DENOM: &str = "unym";
pub const DEFAULT_PREFIX: &str = "n";

#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceRequest {
    pub validator: String,
    pub signer_address: AccountId,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceRequestResponse {
    pub account_number: u64,
    pub sequence: u64,
    pub chain_id: tendermint::chain::Id,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct BroadcastRequest {
    pub base58_tx_bytes: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BroadcastResponse {
    pub tx_hash: String,
    pub success: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RequestTypes {
    Sequence(SequenceRequest),
    Broadcast(BroadcastRequest),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ResponseTypes {
    Sequence(SequenceRequestResponse),
    Broadcast(BroadcastResponse),
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

// parse returned response from service: ignore empty SURB data packets + parse incoming message to struct or error
// we know we are expecting JSON here but an irl helper would parse conditionally on bytes / string incoming
pub async fn listen_and_parse_response(client: &mut MixnetClient) -> ResponseTypes {
    let mut message: Vec<ReconstructedMessage> = Vec::new();

    // get the actual message - discard the empty vec sent along with the SURB topup request
    while let Some(new_message) = client.wait_for_messages().await {
        if new_message.is_empty() {
            continue;
        }
        message = new_message;
        break;
    }

    // parse vec<u8> -> JSON String
    let mut parsed = String::new();
    if let Some(r) = message.iter().next() {
        parsed = String::from_utf8(r.message.clone()).unwrap(); 
    }
    let sp_response: crate::ResponseTypes = serde_json::from_str(&parsed).unwrap();
    sp_response
}

// parse incoming request: parse incoming message to struct + get sender_tag for SURB reply  
// we know we are expecting JSON here but an irl helper would parse conditionally on bytes / string incoming
pub async fn listen_and_parse_request(client: &mut MixnetClient) -> (RequestTypes, AnonymousSenderTag) {
    let mut message: Vec<ReconstructedMessage> = Vec::new();

    // get the actual message - discard the empty vec sent along with the SURB topup request
    while let Some(new_message) = client.wait_for_messages().await {
        if new_message.is_empty() {
            continue;
        }
        message = new_message;
        break;
    }

    // parse vec<u8> -> JSON String
    let mut parsed = String::new();
    if let Some(r) = message.iter().next() {
        parsed = String::from_utf8(r.message.clone()).unwrap(); 
    }
    let client_request: crate::RequestTypes = serde_json::from_str(&parsed).unwrap();

    // get the sender_tag for anon reply 
    let return_recipient = message[0].sender_tag.unwrap(); 

    (client_request, return_recipient) 
}