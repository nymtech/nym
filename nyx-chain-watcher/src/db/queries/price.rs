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
                (timestamp, chf, usd, eur, gbp, btc)
                VALUES
                ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(timestamp) DO UPDATE SET
            chf=excluded.chf,
            usd=excluded.usd,
            eur=excluded.eur,
            gbp=excluded.gbp,
            btc=excluded.btc;",
        timestamp,
        price_data.nym.chf,
        price_data.nym.usd,
        price_data.nym.eur,
        price_data.nym.gbp,
        price_data.nym.btc,
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub(crate) async fn get_latest_price(pool: &DbPool) -> anyhow::Result<PriceHistory> {
    let result = sqlx::query!(
        "SELECT timestamp, chf, usd, eur, gbp, btc FROM price_history ORDER BY timestamp DESC LIMIT 1;"
    )
    .fetch_one(pool)
    .await?;

    Ok(PriceHistory {
        timestamp: result.timestamp,
        chf: result.chf,
        usd: result.usd,
        eur: result.eur,
        gbp: result.gbp,
        btc: result.btc,
    })
}

pub(crate) async fn get_average_price(pool: &DbPool) -> anyhow::Result<PriceHistory> {
    // now less 1 day
    let earliest_timestamp = Local::now().sub(chrono::Duration::days(1)).timestamp();

    let result = sqlx::query!(
        "SELECT timestamp, chf, usd, eur, gbp, btc FROM price_history WHERE timestamp >= $1;",
        earliest_timestamp
    )
    .fetch_all(pool)
    .await?;

    let mut price = PriceHistory {
        timestamp: Local::now().timestamp(),
        chf: 0f64,
        usd: 0f64,
        eur: 0f64,
        gbp: 0f64,
        btc: 0f64,
    };

    let mut chf_count = 0;
    let mut usd_count = 0;
    let mut eur_count = 0;
    let mut gbp_count = 0;
    let mut btc_count = 0;

    for p in &result {
        if p.chf != 0f64 {
            price.chf += p.chf;
            chf_count += 1;
        }
        if p.usd != 0f64 {
            price.usd += p.usd;
            usd_count += 1;
        }
        if p.eur != 0f64 {
            price.eur += p.eur;
            eur_count += 1;
        }
        if p.gbp != 0f64 {
            price.gbp += p.gbp;
            gbp_count += 1;
        }
        if p.btc != 0f64 {
            price.btc += p.btc;
            btc_count += 1;
        }
    }

    if chf_count > 0 {
        price.chf /= chf_count as f64;
    }
    if usd_count > 0 {
        price.usd /= usd_count as f64;
    }
    if eur_count > 0 {
        price.eur /= eur_count as f64;
    }
    if gbp_count > 0 {
        price.gbp /= gbp_count as f64;
    }
    if btc_count > 0 {
        price.btc /= btc_count as f64;
    }

    Ok(price)
}