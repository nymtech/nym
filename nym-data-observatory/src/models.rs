use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa::r#gen::serde_json;

#[derive(Serialize, Deserialize, Clone, JsonSchema, ToSchema)]
pub struct WebhookPayload {
    pub height: u64,
    pub transaction_hash: String,
    pub message_index: u64,
    pub message: Option<serde_json::Value>,
}

pub mod openapi_schema {
    use super::*;

    #[derive(ToSchema)]
    pub struct Coin {
        pub denom: String,
        pub amount: String,
    }
}
