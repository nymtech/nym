use nym_validator_client::nyxd::CosmWasmCoin;
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, JsonSchema, ToSchema)]
pub struct WebhookPayload {
    pub transaction_hash: String,
    pub message_index: u64,
    pub sender_address: String,
    pub receiver_address: String,
    pub funds: CosmWasmCoin,
    pub height: u128,
    pub memo: Option<String>,
}
