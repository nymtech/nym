// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};

use crate::statistics::{StatsData, StatsMessage};
use crate::storage::NetworkRequesterStorage;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MixnetStatisticsRequest {
    // date, RFC 3339 format
    since: String,
    // date, RFC 3339 format
    until: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MixnetStatisticsResponse {
    data: Vec<StatsMessage>,
}

#[rocket::post("/mixnet-statistics", data = "<mixnet_statistics_request>")]
pub(crate) async fn post_mixnet_statistics(
    mixnet_statistics_request: Json<MixnetStatisticsRequest>,
    storage: &State<NetworkRequesterStorage>,
) -> Json<MixnetStatisticsResponse> {
    let mixnet_statistics = storage
        .get_service_statistics_in_interval(
            &mixnet_statistics_request.since,
            &mixnet_statistics_request.until,
        )
        .await
        .unwrap()
        .into_iter()
        .map(|data| StatsMessage {
            description: data.service_description,
            request_data: StatsData::new(data.request_processed_bytes as u32),
            response_data: StatsData::new(data.response_processed_bytes as u32),
            interval_seconds: data.interval_seconds as u32,
            timestamp: data.timestamp.to_string(),
        })
        .collect();

    Json(MixnetStatisticsResponse {
        data: mixnet_statistics,
    })
}
