use time::UtcDateTime;

use crate::db::{models::NetworkSummary, DbPool};

/// take `last_updated` instead of calculating it so that `summary` matches
/// `daily_summary`
pub(crate) async fn insert_summaries(
    pool: &DbPool,
    summaries: Vec<(String, usize)>,
    summary: NetworkSummary,
    last_updated: UtcDateTime,
) -> anyhow::Result<()> {
    insert_summary(pool, summaries, last_updated).await?;

    insert_summary_history(pool, summary, last_updated).await?;

    Ok(())
}

async fn insert_summary(
    pool: &DbPool,
    summaries: Vec<(String, usize)>,
    last_updated: UtcDateTime,
) -> anyhow::Result<()> {
    let timestamp = last_updated.unix_timestamp();
    let mut tx = pool.begin().await?;

    for (kind, value) in summaries {
        let value = value.to_string();
        crate::db::query(
            "INSERT INTO summary
                    (key, value_json, last_updated_utc)
                    VALUES (?, ?, ?)
                    ON CONFLICT(key) DO UPDATE SET
                    value_json=excluded.value_json,
                    last_updated_utc=excluded.last_updated_utc;",
        )
        .bind(kind.clone())
        .bind(value)
        .bind(timestamp)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            tracing::error!("Failed to insert data for {kind}: {err}, aborting transaction",);
            err
        })?;
    }

    tx.commit().await?;

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
    summary: NetworkSummary,
    last_updated: UtcDateTime,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;

    let value_json = serde_json::to_string(&summary)?;
    let timestamp = last_updated.unix_timestamp();

    let date = datetime_to_only_date_str(last_updated);

    crate::db::query(
        "INSERT INTO summary_history
                (date, timestamp_utc, value_json)
                VALUES (?, ?, ?)
                ON CONFLICT(date) DO UPDATE SET
                timestamp_utc=excluded.timestamp_utc,
                value_json=excluded.value_json;",
    )
    .bind(date)
    .bind(timestamp)
    .bind(value_json)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// YYYY-MM-DD, without time
fn datetime_to_only_date_str(datetime: UtcDateTime) -> String {
    datetime.date().to_string()
}

#[cfg(test)]
mod test {
    use time::macros::utc_datetime;

    use super::*;

    #[test]
    fn date_is_expected_format() {
        // to store records daily, we rely on a date in a specific format
        // if the behaviour changes, you should adjust code to return the expected form
        let example_date = utc_datetime!(2025-05-01 01:23);
        let expected = "2025-05-01";
        assert_eq!(expected, datetime_to_only_date_str(example_date));
    }
}
