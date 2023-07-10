use cosmrs::rpc::{HttpClient, Id};
use serde::{Deserialize, Serialize}; 
use cosmrs::{tx, AccountId, Coin, Denom};
use nym_validator_client::nyxd::cosmwasm_client::types::SequenceResponse;

#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceRequest {
    pub validator: String, 
    pub signer_address: AccountId, 
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceRequestResponse {
    pub sequence: u8, // fix this - should be cosmwasmclient::SequenceResponse  
    pub chain_id: Id
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