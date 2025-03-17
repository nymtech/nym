use crate::db::{
    models::{CoingeckoPriceResponse, PriceRecord},
    queries::price::insert_nym_prices,
};
use core::str;
use tokio::time::Duration;

use crate::db::DbPool;
use crate::http::state::PriceScraperState;

const REFRESH_DELAY: Duration = Duration::from_secs(300);
const FAILURE_RETRY_DELAY: Duration = Duration::from_secs(60 * 2);
const COINGECKO_API_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=nym&vs_currencies=chf,usd,eur,gbp,btc";

pub(crate) struct PriceScraper {
    shared_state: PriceScraperState,
    db_pool: DbPool,
}

impl PriceScraper {
    pub(crate) fn new(shared_state: PriceScraperState, db_pool: DbPool) -> Self {
        PriceScraper {
            shared_state,
            db_pool,
        }
    }

    async fn get_coingecko_prices(&self) -> anyhow::Result<CoingeckoPriceResponse> {
        tracing::info!("💰 Fetching CoinGecko prices from {COINGECKO_API_URL}");

        let response = reqwest::get(COINGECKO_API_URL)
            .await?
            .json::<CoingeckoPriceResponse>()
            .await;

        tracing::info!("Got response {:?}", response);
        match response {
            Ok(resp) => {
                let price_record = PriceRecord {
                    timestamp: time::OffsetDateTime::now_utc().unix_timestamp(),
                    nym: resp.nym.clone(),
                };

                insert_nym_prices(&self.db_pool, price_record).await?;
                Ok(resp)
            }
            Err(err) => {
                //tracing::info!("💰 CoinGecko price response: {:?}", response);
                tracing::error!("Error sending request: {err}");
                Err(err.into())
            }
        }
    }

    pub(crate) async fn run(&self) {
        loop {
            tracing::info!("Running in a loop 🏃");
            match self.get_coingecko_prices().await {
                Ok(coingecko_price_response) => {
                    self.shared_state
                        .new_success(coingecko_price_response)
                        .await;
                    tracing::info!("✅ Successfully fetched CoinGecko prices");
                    tokio::time::sleep(REFRESH_DELAY).await;
                }
                Err(err) => {
                    tracing::error!("❌ Failed to get CoinGecko prices: {err}");
                    tracing::info!("Retrying in {}s...", FAILURE_RETRY_DELAY.as_secs());
                    self.shared_state.new_failure(err.to_string()).await;
                    tokio::time::sleep(FAILURE_RETRY_DELAY).await;
                }
            }
        }
    }
}
