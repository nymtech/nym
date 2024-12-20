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
    #[schema(value_type = openapi_schema::Coin)]
    pub funds: CosmWasmCoin,
    pub height: u128,
    pub memo: Option<String>,
}

pub mod openapi_schema {
    use super::*;

    #[derive(ToSchema)]
    pub struct Coin {
        pub denom: String,
        pub amount: String,
    }
}
