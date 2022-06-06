// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};

use crate::storage::NetworkStatisticsStorage;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MixnetStatisticsRequest {
    // date, RFC 3339 format
    since: String,
    // date, RFC 3339 format
    until: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServiceStatisticsResponse {
    pub requested_service: String,
    pub request_processed_bytes: u32,
    pub response_processed_bytes: u32,
    pub interval_seconds: u32,
    pub timestamp: String,
}

#[rocket::post("/service-statistics", data = "<service_statistics_request>")]
pub(crate) async fn post_service_statistics(
    service_statistics_request: Json<MixnetStatisticsRequest>,
    storage: &State<NetworkStatisticsStorage>,
) -> Json<Vec<ServiceStatisticsResponse>> {
    let service_statistics = storage
        .get_service_statistics_in_interval(
            &service_statistics_request.since,
            &service_statistics_request.until,
        )
        .await
        .unwrap()
        .into_iter()
        .map(|data| ServiceStatisticsResponse {
            requested_service: data.requested_service,
            request_processed_bytes: data.request_processed_bytes as u32,
            response_processed_bytes: data.response_processed_bytes as u32,
            interval_seconds: data.interval_seconds as u32,
            timestamp: data.timestamp.to_string(),
        })
        .collect();

    Json(service_statistics)
}
