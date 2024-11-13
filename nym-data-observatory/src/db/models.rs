use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct CurrencyPrices {
    pub(crate) chf: f32,
    pub(crate) usd: f32,
    pub(crate) eur: f32,
    pub(crate) btc: f32,
}

// Struct to hold Coingecko response
#[derive(Clone, Deserialize, Debug, ToSchema)]
pub(crate) struct CoingeckoPriceResponse {
    pub(crate) nym: CurrencyPrices,
}

#[derive(Clone, Deserialize, Debug, ToSchema)]
pub(crate) struct PriceRecord {
    pub(crate) timestamp: i64,
    pub(crate) nym: CurrencyPrices,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub(crate) struct PriceHistory {
    pub(crate) timestamp: i64,
    pub(crate) chf: f32,
    pub(crate) usd: f32,
    pub(crate) eur: f32,
    pub(crate) btc: f32,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub(crate) struct PaymentRecord {
    pub(crate) transaction_hash: String,
    pub(crate) sender_address: String,
    pub(crate) receiver_address: String,
    pub(crate) amount: f64,
    pub(crate) timestamp: i64,
    pub(crate) height: i64,
}
