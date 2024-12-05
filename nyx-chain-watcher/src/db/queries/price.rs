use crate::db::models::{PriceHistory, PriceRecord};
use crate::db::DbPool;

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
        chf: result.chf as f32,
        usd: result.usd as f32,
        eur: result.eur as f32,
        btc: result.btc as f32,
    })
}
