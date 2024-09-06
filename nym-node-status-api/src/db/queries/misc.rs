use crate::db::DbPool;

pub(crate) async fn insert_with_tx(
    key: &str,
    value: &usize,
    last_updated_utc: &i64,
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> anyhow::Result<()> {
    let value_json = format!("{}", value);
    sqlx::query!(
        "INSERT INTO summary
                (key, value_json, last_updated_utc)
                VALUES (?, ?, ?)
                ON CONFLICT(key) DO UPDATE SET
                value_json=excluded.value_json,
                last_updated_utc=excluded.last_updated_utc;",
        key,
        value_json,
        last_updated_utc
    )
    .execute(&mut **transaction)
    .await?;

    Ok(())
}

pub(crate) async fn insert_summary(
    pool: &DbPool,
    kv_pairs: &Vec<(&str, usize)>,
) -> anyhow::Result<()> {
    let last_updated_utc = chrono::offset::Utc::now().timestamp();
    let mut tx = pool.begin().await?;

    for (key, value) in kv_pairs {
        let value_json = value.to_string();
        sqlx::query!(
            "INSERT INTO summary
                    (key, value_json, last_updated_utc)
                    VALUES (?, ?, ?)
                    ON CONFLICT(key) DO UPDATE SET
                    value_json=excluded.value_json,
                    last_updated_utc=excluded.last_updated_utc;",
            key,
            value_json,
            last_updated_utc
        )
        .execute(&mut tx)
        .await?;
    }

    Ok(())
}

/// keep daily summary
pub(crate) async fn insert_summary_history(record: &[(&str, usize)]) -> anyhow::Result<()> {
    // let summary = NetworkSummary {
    //     mixnodes: MixnodeSummary {
    //         bonded: (),
    //         blacklisted: (),
    //         historical: (),
    //     },
    //     gateways: todo!(),
    // };
    // let value_json = serde_json::to_string(network_summary)?;
    unimplemented!()
}
