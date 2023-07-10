use cosmrs::rpc::{Id};
use serde::{Deserialize, Serialize}; 
use cosmrs::{AccountId};

#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceRequest {
    pub validator: String, 
    pub signer_address: AccountId, 
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceResponse {
    pub sequence_response: u8, 
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
    Sequence(SequenceResponse), 
    Broadcast(BroadcastResponse)
}

