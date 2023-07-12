use serde::{Deserialize, Serialize}; 
use cosmrs::{AccountId, tendermint};

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