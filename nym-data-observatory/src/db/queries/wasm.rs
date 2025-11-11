use crate::db::DbPool;
use anyhow::Result;
use nyxd_scraper_psql::models::DbCoin;
use serde_json::Value;
use time::PrimitiveDateTime;

#[allow(clippy::too_many_arguments)]
pub async fn insert_wasm_execute(
    pool: &DbPool,
    sender: String,
    contract_address: String,
    message_type: String,
    raw_contract_message: &Value,
    funds: Option<Vec<DbCoin>>,
    executed_at: PrimitiveDateTime,
    height: i64,
    hash: String,
    message_index: i64,
    memo: String,
    fee: &Vec<DbCoin>,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO wasm_execute_contract (
            sender,
            contract_address,
            message_type,
            raw_contract_message,
            funds,
            executed_at,
            height,
            hash,
            message_index,
            memo,
            fee
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
        sender,
        contract_address,
        message_type,
        raw_contract_message,
        &funds as _,
        executed_at,
        height,
        hash,
        message_index,
        memo,
        &fee as _,
    )
    .execute(pool)
    .await?;

    Ok(())
}
