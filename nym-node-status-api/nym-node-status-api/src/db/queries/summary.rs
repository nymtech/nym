use chrono::{DateTime, Utc};
use futures_util::TryStreamExt;
use std::collections::HashMap;
use tracing::error;

use crate::{
    db::{
        models::{
            gateway::{GatewaySummary, GatewaySummaryBonded, GatewaySummaryHistorical},
            mixnode::{MixnodeSummary, MixnodeSummaryBonded, MixnodeSummaryHistorical},
            NetworkSummary, SummaryDto, SummaryHistoryDto, GATEWAYS_BONDED_COUNT,
            GATEWAYS_HISTORICAL_COUNT, LEGACY_MIXNODES_COUNT, MIXNODES_BONDED_ACTIVE,
            MIXNODES_BONDED_COUNT, MIXNODES_HISTORICAL_COUNT,
        },
        DbPool,
    },
    http::{
        error::{HttpError, HttpResult},
        models::SummaryHistory,
    },
};

pub(crate) async fn get_summary_history(pool: &DbPool) -> anyhow::Result<Vec<SummaryHistory>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        SummaryHistoryDto,
        r#"SELECT
            id as "id!",
            date as "date!",
            timestamp_utc as "timestamp_utc!",
            value_json as "value_json!"
         FROM summary_history
         ORDER BY date DESC
         LIMIT 30"#,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<_>>()
    .await?;

    let items = items
        .into_iter()
        .map(|item| item.try_into())
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(|e| {
            error!("Conversion from DTO failed: {e}. Invalidly stored data?");
            e
        })?;
    Ok(items)
}

async fn get_summary_dto(pool: &DbPool) -> anyhow::Result<Vec<SummaryDto>> {
    let mut conn = pool.acquire().await?;
    Ok(sqlx::query_as!(
        SummaryDto,
        r#"SELECT
            key as "key!",
            value_json as "value_json!",
            last_updated_utc as "last_updated_utc!"
         FROM summary"#
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<_>>()
    .await?)
}

pub(crate) async fn get_summary(pool: &DbPool) -> HttpResult<NetworkSummary> {
    let items = get_summary_dto(pool).await.map_err(|err| {
        tracing::error!("Couldn't get Summary from DB: {err}");
        HttpError::internal()
    })?;
    from_summary_dto(items).await
}

async fn from_summary_dto(items: Vec<SummaryDto>) -> HttpResult<NetworkSummary> {
    // convert database rows into a map by key
    let mut map = HashMap::new();
    for item in items {
        map.insert(item.key.clone(), item);
    }

    // check we have all the keys we are expecting, and build up a map of errors for missing one
    let keys = [
        GATEWAYS_BONDED_COUNT,
        GATEWAYS_HISTORICAL_COUNT,
        MIXNODES_BONDED_ACTIVE,
        MIXNODES_BONDED_COUNT,
        LEGACY_MIXNODES_COUNT,
        MIXNODES_HISTORICAL_COUNT,
    ];

    let mut errors: Vec<&str> = vec![];
    for key in keys {
        if !map.contains_key(key) {
            errors.push(key);
        }
    }

    // return an error if anything is missing, with a nice list
    if !errors.is_empty() {
        tracing::error!("Summary value missing: {}", errors.join(", "));
        return Err(HttpError::internal());
    }

    // strip the options and use default values (anything missing is trapped above)
    let mixnodes_bonded_count: SummaryDto =
        map.get(MIXNODES_BONDED_COUNT).cloned().unwrap_or_default();
    let mixnodes_bonded_active: SummaryDto =
        map.get(MIXNODES_BONDED_ACTIVE).cloned().unwrap_or_default();
    let legacy_mixnodes_count: SummaryDto =
        map.get(LEGACY_MIXNODES_COUNT).cloned().unwrap_or_default();
    let gateways_bonded_count: SummaryDto =
        map.get(GATEWAYS_BONDED_COUNT).cloned().unwrap_or_default();
    let mixnodes_historical_count: SummaryDto = map
        .get(MIXNODES_HISTORICAL_COUNT)
        .cloned()
        .unwrap_or_default();
    let gateways_historical_count: SummaryDto = map
        .get(GATEWAYS_HISTORICAL_COUNT)
        .cloned()
        .unwrap_or_default();

    Ok(NetworkSummary {
        mixnodes: MixnodeSummary {
            bonded: MixnodeSummaryBonded {
                count: to_count_i32(&mixnodes_bonded_count),
                active: to_count_i32(&mixnodes_bonded_active),
                legacy: to_count_i32(&legacy_mixnodes_count),
                last_updated_utc: to_timestamp(&mixnodes_bonded_count),
            },
            historical: MixnodeSummaryHistorical {
                count: to_count_i32(&mixnodes_historical_count),
                last_updated_utc: to_timestamp(&mixnodes_historical_count),
            },
        },
        gateways: GatewaySummary {
            bonded: GatewaySummaryBonded {
                count: to_count_i32(&gateways_bonded_count),
                last_updated_utc: to_timestamp(&gateways_bonded_count),
            },
            historical: GatewaySummaryHistorical {
                count: to_count_i32(&gateways_historical_count),
                last_updated_utc: to_timestamp(&gateways_historical_count),
            },
        },
    })
}

fn to_count_i32(value: &SummaryDto) -> i32 {
    value.value_json.parse::<i32>().unwrap_or_default()
}

fn to_timestamp(value: &SummaryDto) -> String {
    timestamp_as_utc(value.last_updated_utc as u64).to_rfc3339()
}

fn timestamp_as_utc(unix_timestamp: u64) -> DateTime<Utc> {
    let d = std::time::UNIX_EPOCH + std::time::Duration::from_secs(unix_timestamp);
    d.into()
}
