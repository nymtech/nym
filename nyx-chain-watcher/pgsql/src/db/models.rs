use anyhow::Context;
use nym_validator_client::nyxd::Coin;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
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

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub(crate) struct PaymentRecord {
    pub(crate) transaction_hash: String,
    pub(crate) sender_address: String,
    pub(crate) receiver_address: String,
    pub(crate) amount: f64,
    pub(crate) timestamp: i64,
    pub(crate) height: i64,
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub(crate) struct Transaction {
    pub(crate) id: i64,
    pub(crate) tx_hash: String,
    pub(crate) height: i64,
    pub(crate) message_index: i64,
    pub(crate) sender: String,
    pub(crate) recipient: String,
    pub(crate) amount: String,
    pub(crate) memo: Option<String>,
    pub(crate) created_at: Option<OffsetDateTime>,
}

impl Transaction {
    pub(crate) fn funds(&self) -> anyhow::Result<Coin> {
        self.amount
            .as_str()
            .parse()
            .context("failed to parse transaction amount")
    }
}
