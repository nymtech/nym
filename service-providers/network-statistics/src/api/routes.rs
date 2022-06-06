// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};

use crate::api::error::Result;
use crate::storage::NetworkStatisticsStorage;
use crate::storage::StatsMessage;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServiceStatisticsRequest {
    // date, RFC 3339 format
    since: String,
    // date, RFC 3339 format
    until: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServiceStatistic {
    pub requested_service: String,
    pub request_processed_bytes: u32,
    pub response_processed_bytes: u32,
    pub interval_seconds: u32,
    pub timestamp: String,
}

#[rocket::post("/service-statistics", data = "<service_statistics_request>")]
pub(crate) async fn post_service_statistics(
    service_statistics_request: Json<ServiceStatisticsRequest>,
    storage: &State<NetworkStatisticsStorage>,
) -> Result<Json<Vec<ServiceStatistic>>> {
    let service_statistics = storage
        .get_service_statistics_in_interval(
            &service_statistics_request.since,
            &service_statistics_request.until,
        )
        .await?
        .into_iter()
        .map(|data| ServiceStatistic {
            requested_service: data.requested_service,
            request_processed_bytes: data.request_processed_bytes as u32,
            response_processed_bytes: data.response_processed_bytes as u32,
            interval_seconds: data.interval_seconds as u32,
            timestamp: data.timestamp.to_string(),
        })
        .collect();

    Ok(Json(service_statistics))
}

#[rocket::post("/statistics", data = "<statistics>")]
pub(crate) async fn post_statistic(
    statistics: Json<StatsMessage>,
    storage: &State<NetworkStatisticsStorage>,
) -> Result<Json<()>> {
    storage.insert_service_statistics(statistics.0).await?;
    Ok(Json(()))
}
