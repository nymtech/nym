use crate::db::{
    models::{CoingeckoPriceResponse, PriceRecord},
    queries::price::insert_nym_prices,
};
use core::str;
use tokio::task::JoinHandle;
use tokio::time::Duration;

use crate::db::DbPool;

const REFRESH_DELAY: Duration = Duration::from_secs(300);
const FAILURE_RETRY_DELAY: Duration = Duration::from_secs(60 * 2);
const COINGECKO_API_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=nym&vs_currencies=chf,usd,eur,gbp,btc";

pub(crate) async fn run_price_scraper(db_pool: &DbPool) -> JoinHandle<()> {
    loop {
        tracing::info!("Running in a loop 🏃");
        if let Err(e) = get_coingecko_prices(db_pool).await {
            tracing::error!("❌ Failed to get CoinGecko prices: {e}");
            tracing::info!("Retrying in {}s...", FAILURE_RETRY_DELAY.as_secs());
            tokio::time::sleep(FAILURE_RETRY_DELAY).await;
        } else {
            tracing::info!("✅ Successfully fetched CoinGecko prices");
            tokio::time::sleep(REFRESH_DELAY).await;
        }
    }
}

async fn get_coingecko_prices(pool: &DbPool) -> anyhow::Result<()> {
    tracing::info!("💰 Fetching CoinGecko prices from {}", COINGECKO_API_URL);

    let response = reqwest::get(COINGECKO_API_URL)
        .await?
        .json::<CoingeckoPriceResponse>()
        .await;

    tracing::info!("Got response {:?}", response);
    match response {
        Ok(resp) => {
            let price_record = PriceRecord {
                timestamp: time::OffsetDateTime::now_utc().unix_timestamp(),
                nym: resp.nym,
            };

            insert_nym_prices(pool, price_record).await?;
        }
        Err(e) => {
            //tracing::info!("💰 CoinGecko price response: {:?}", response);
            tracing::error!("Error sending request: {}", e);
        }
    }

    Ok(())
}
