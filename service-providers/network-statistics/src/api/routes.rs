// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// due to the macro expansion of rather old rocket macros...
#![allow(unused_imports)]

use crate::api::error::Result;
use crate::storage::NetworkStatisticsStorage;
use nym_statistics_common::StatsMessage;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StatisticsRequest {
    // date, RFC 3339 format
    since: String,
    // date, RFC 3339 format
    until: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum GenericStatistic {
    Service(ServiceStatistic),
    Gateway(GatewayStatistic),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServiceStatistic {
    pub requested_service: String,
    pub request_processed_bytes: u32,
    pub response_processed_bytes: u32,
    pub interval_seconds: u32,
    pub timestamp: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GatewayStatistic {
    pub gateway_id: String,
    pub inbox_count: u32,
    pub timestamp: String,
}

#[rocket::post("/all-statistics", data = "<all_statistics_request>")]
pub(crate) async fn post_all_statistics(
    all_statistics_request: Json<StatisticsRequest>,
    storage: &State<NetworkStatisticsStorage>,
) -> Result<Json<Vec<GenericStatistic>>> {
    let all_statistics = storage
        .get_service_statistics_in_interval(
            &all_statistics_request.since,
            &all_statistics_request.until,
        )
        .await?
        .into_iter()
        .map(|data| {
            GenericStatistic::Service(ServiceStatistic {
                requested_service: data.requested_service,
                request_processed_bytes: data.request_processed_bytes as u32,
                response_processed_bytes: data.response_processed_bytes as u32,
                interval_seconds: data.interval_seconds as u32,
                timestamp: data.timestamp.to_string(),
            })
        })
        .chain(
            storage
                .get_gateway_statistics_in_interval(
                    &all_statistics_request.since,
                    &all_statistics_request.until,
                )
                .await?
                .into_iter()
                .map(|data| {
                    GenericStatistic::Gateway(GatewayStatistic {
                        gateway_id: data.gateway_id,
                        inbox_count: data.inbox_count as u32,
                        timestamp: data.timestamp.to_string(),
                    })
                }),
        )
        .collect();

    Ok(Json(all_statistics))
}

#[rocket::post("/statistic", data = "<statistic>")]
pub(crate) async fn post_statistic(
    statistic: Json<StatsMessage>,
    storage: &State<NetworkStatisticsStorage>,
) -> Result<Json<()>> {
    storage.insert_statistics(statistic.0).await?;
    Ok(Json(()))
}
