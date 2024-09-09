use crate::db::{models::NetworkSummary, DbPool};
use chrono::{DateTime, Utc};

/// take `last_updated` instead of calculating it so that `summary` matches
/// `daily_summary`
pub(crate) async fn insert_summaries(
    pool: &DbPool,
    summaries: &[(&str, &usize)],
    summary: &NetworkSummary,
    last_updated: DateTime<Utc>,
) -> anyhow::Result<()> {
    insert_summary(pool, summaries, last_updated).await?;

    insert_summary_history(pool, summary, last_updated).await?;

    Ok(())
}

async fn insert_summary(
    pool: &DbPool,
    summaries: &[(&str, &usize)],
    last_updated: DateTime<Utc>,
) -> anyhow::Result<()> {
    let timestamp = last_updated.timestamp();
    let mut tx = pool.begin().await?;

    for (kind, value) in summaries {
        let value = value.to_string();
        sqlx::query!(
            "INSERT INTO summary
                    (key, value_json, last_updated_utc)
                    VALUES (?, ?, ?)
                    ON CONFLICT(key) DO UPDATE SET
                    value_json=excluded.value_json,
                    last_updated_utc=excluded.last_updated_utc;",
            kind,
            value,
            timestamp
        )
        .execute(&mut tx)
        .await
        .map_err(|err| {
            tracing::error!("Failed to insert data for {kind}: {err}, aborting transaction",);
            err
        })?;
    }

    Ok(())
}

/// For `<date_N>`, `summary_history` is updated with fresh data on every
/// iteration.
///
/// After UTC midnight, summary is inserted for `<date_N+1>` and last entry for
/// `<date_N>` stays there forever.
///
/// This is not aggregate data, it's a set of latest data points
async fn insert_summary_history(
    pool: &DbPool,
    summary: &NetworkSummary,
    last_updated: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;

    let value_json = serde_json::to_string(&summary)?;
    let timestamp = last_updated.timestamp();
    let now_rfc3339 = last_updated.to_rfc3339();
    // YYYY-MM-DD, without time
    let date = &now_rfc3339[..10];

    sqlx::query!(
        "INSERT INTO summary_history
                (date, timestamp_utc, value_json)
                VALUES (?, ?, ?)
                ON CONFLICT(date) DO UPDATE SET
                timestamp_utc=excluded.timestamp_utc,
                value_json=excluded.value_json;",
        date,
        timestamp,
        value_json
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}
