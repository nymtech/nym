use crate::db::DbPool;
use anyhow::{Context, Result};
use regex::Regex;
use crate::db::models::PaymentRecord;

pub async fn get_last_checked_height(pool: &DbPool) -> Result<i64> {
    let result = sqlx::query_scalar!("SELECT MAX(height) FROM payments")
        .fetch_one(pool)
        .await?;
    Ok(result.unwrap_or(0))
}

lazy_static::lazy_static! {
    static ref HEX_PATTERN: Regex = Regex::new(r"^[A-Fa-f0-9]{64}$").unwrap();
    static ref BASE64_PATTERN: Regex = Regex::new(r"^[A-Za-z0-9+/=]+$").unwrap();
}

pub async fn get_transaction_record(pool: &DbPool, record_txs: &str) -> Result<Option<PaymentRecord>> {
    let query = if HEX_PATTERN.is_match(record_txs) {
        "SELECT transaction_hash, sender_address, receiver_address, amount, timestamp, height
         FROM transactions WHERE tx_hash = $1"
    } else if BASE64_PATTERN.is_match(record_txs) {
        "SELECT transaction_hash, sender_address, receiver_address, amount, timestamp, height
         FROM transactions WHERE memo LIKE $1"
    } else {
        return Ok(None);
    };

    let param = if BASE64_PATTERN.is_match(record_txs) {
        format!("%{}%", record_txs)
    } else {
        record_txs.to_string()
    };

    sqlx::query_as::<_, PaymentRecord>(query)
        .bind(param)
        .fetch_optional(pool)
        .await
        .context("Database query failed")
}

pub async fn insert_payment(
    pool: &DbPool,
    transaction_hash: String,
    sender_address: String,
    receiver_address: String,
    amount: f64,
    height: i64,
    memo: Option<String>,
) -> Result<()> {
    let timestamp = chrono::Utc::now().timestamp();

    sqlx::query!(
        r#"
        INSERT INTO payments (
            transaction_hash, sender_address, receiver_address,
            amount, height, timestamp, memo
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        transaction_hash,
        sender_address,
        receiver_address,
        amount,
        height,
        timestamp,
        memo,
    )
    .execute(pool)
    .await?;

    Ok(())
}
