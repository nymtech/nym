use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, Debug, ToSchema)]
pub(crate) struct CurrencyPrices {
    pub(crate) chf: f64,
    pub(crate) usd: f64,
    pub(crate) eur: f64,
    pub(crate) gbp: f64,
    pub(crate) btc: f64,
}

// Struct to hold Coingecko response
#[derive(Clone, Serialize, Deserialize, Debug, ToSchema)]
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
    pub(crate) chf: f64,
    pub(crate) usd: f64,
    pub(crate) eur: f64,
    pub(crate) gbp: f64,
    pub(crate) btc: f64,
}
