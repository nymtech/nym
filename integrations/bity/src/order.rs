use cosmrs::crypto::PublicKey;
use cosmrs::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderSignature {
    pub public_key: PublicKey,
    pub account_id: AccountId,
    pub signature_as_hex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    pub account_id: AccountId,
    pub message: String,
    pub signature: OrderSignature,
}
