// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    CoreNodeStatus, ErrorResponse, GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport,
    MixnodeUptimeHistory,
};
use crate::storage::ValidatorApiStorage;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;

#[get("/mixnode/<pubkey>/report")]
pub(crate) async fn mixnode_report(
    storage: &State<ValidatorApiStorage>,
    pubkey: &str,
) -> Result<Json<MixnodeStatusReport>, ErrorResponse> {
    storage
        .construct_mixnode_report(pubkey)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err, Status::NotFound))
}

#[get("/gateway/<pubkey>/report")]
pub(crate) async fn gateway_report(
    storage: &State<ValidatorApiStorage>,
    pubkey: &str,
) -> Result<Json<GatewayStatusReport>, ErrorResponse> {
    storage
        .construct_gateway_report(pubkey)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err, Status::NotFound))
}

#[get("/mixnode/<pubkey>/history")]
pub(crate) async fn mixnode_uptime_history(
    storage: &State<ValidatorApiStorage>,
    pubkey: &str,
) -> Result<Json<MixnodeUptimeHistory>, ErrorResponse> {
    storage
        .get_mixnode_uptime_history(pubkey)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err, Status::NotFound))
}

#[get("/gateway/<pubkey>/history")]
pub(crate) async fn gateway_uptime_history(
    storage: &State<ValidatorApiStorage>,
    pubkey: &str,
) -> Result<Json<GatewayUptimeHistory>, ErrorResponse> {
    storage
        .get_gateway_uptime_history(pubkey)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err, Status::NotFound))
}

#[get("/mixnode/<pubkey>/core-status-count?<since>")]
pub(crate) async fn mixnode_core_status_count(
    storage: &State<ValidatorApiStorage>,
    pubkey: &str,
    since: Option<i64>,
) -> Json<CoreNodeStatus> {
    let count = storage
        .get_core_mixnode_status_count(pubkey, since)
        .await
        .unwrap_or_default();

    Json(CoreNodeStatus {
        identity: pubkey.to_string(),
        count,
    })
}

#[get("/gateway/<pubkey>/core-status-count?<since>")]
pub(crate) async fn gateway_core_status_count(
    storage: &State<ValidatorApiStorage>,
    pubkey: &str,
    since: Option<i64>,
) -> Json<CoreNodeStatus> {
    let count = storage
        .get_core_gateway_status_count(pubkey, since)
        .await
        .unwrap_or_default();

    Json(CoreNodeStatus {
        identity: pubkey.to_string(),
        count,
    })
}
