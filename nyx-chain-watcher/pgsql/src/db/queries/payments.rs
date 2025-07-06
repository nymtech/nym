use crate::db::DbPool;
use anyhow::Result;

pub async fn get_last_checked_height(pool: &DbPool) -> Result<i64> {
    let result = sqlx::query_scalar!("SELECT MAX(height) FROM payments")
        .fetch_one(pool)
        .await?;
    Ok(result.unwrap_or(0) as i64)
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
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
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
