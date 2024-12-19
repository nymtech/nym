use crate::db::models::{PriceHistory, PriceRecord};
use crate::db::DbPool;
use chrono::Local;
use std::ops::Sub;

pub(crate) async fn insert_nym_prices(
    pool: &DbPool,
    price_data: PriceRecord,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    let timestamp = price_data.timestamp;
    sqlx::query!(
        "INSERT INTO price_history
                (timestamp, chf, usd, eur, btc)
                VALUES
                ($1, $2, $3, $4, $5)
            ON CONFLICT(timestamp) DO UPDATE SET
            chf=excluded.chf,
            usd=excluded.usd,
            eur=excluded.eur,
            btc=excluded.btc;",
        timestamp,
        price_data.nym.chf,
        price_data.nym.usd,
        price_data.nym.eur,
        price_data.nym.btc,
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub(crate) async fn get_latest_price(pool: &DbPool) -> anyhow::Result<PriceHistory> {
    let result = sqlx::query!(
        "SELECT timestamp, chf, usd, eur, btc FROM price_history ORDER BY timestamp DESC LIMIT 1;"
    )
    .fetch_one(pool)
    .await?;

    Ok(PriceHistory {
        timestamp: result.timestamp,
        chf: result.chf,
        usd: result.usd,
        eur: result.eur,
        btc: result.btc,
    })
}

pub(crate) async fn get_average_price(pool: &DbPool) -> anyhow::Result<PriceHistory> {
    // now less 1 day
    let earliest_timestamp = Local::now().sub(chrono::Duration::days(1)).timestamp();

    let result = sqlx::query!(
        "SELECT timestamp, chf, usd, eur, btc FROM price_history WHERE timestamp >= $1;",
        earliest_timestamp
    )
    .fetch_all(pool)
    .await?;

    let count = result.len() as f64;

    let mut price = PriceHistory {
        timestamp: Local::now().timestamp(),
        chf: 0f64,
        usd: 0f64,
        eur: 0f64,
        btc: 0f64,
    };

    for p in &result {
        price.chf += p.chf;
        price.usd += p.usd;
        price.eur += p.eur;
        price.btc += p.btc;
    }

    if count > 0f64 {
        price.chf /= count;
        price.usd /= count;
        price.eur /= count;
        price.btc /= count;
    }

    Ok(price)
}
