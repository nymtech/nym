// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::local_guard::LocalRequest;
use crate::node_status_api::models::{
    ErrorResponse, GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport,
    MixnodeUptimeHistory,
};
use crate::storage::NodeStatusStorage;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;

#[get("/daily-chores")]
pub(crate) async fn rewarding_chores(
    _local_request: LocalRequest,
    storage: &State<NodeStatusStorage>,
) -> Result<&'static str, ErrorResponse> {
    if storage
        .daily_chores()
        .await
        .map_err(|err| ErrorResponse::new(err, Status::InternalServerError))?
    {
        Ok("Updated historical uptimes and purged old reports.")
    } else {
        Ok("The historical uptimes were already updated at some point today - nothing was done now.")
    }
}

#[get("/mixnode/<pubkey>/report")]
pub(crate) async fn mixnode_report(
    storage: &State<NodeStatusStorage>,
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
    storage: &State<NodeStatusStorage>,
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
    storage: &State<NodeStatusStorage>,
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
    storage: &State<NodeStatusStorage>,
    pubkey: &str,
) -> Result<Json<GatewayUptimeHistory>, ErrorResponse> {
    storage
        .get_gateway_uptime_history(pubkey)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err, Status::NotFound))
}

#[get("/mixnodes/all/report")]
pub(crate) async fn mixnodes_full_report(
    storage: &State<NodeStatusStorage>,
) -> Result<Json<Vec<MixnodeStatusReport>>, ErrorResponse> {
    storage
        .get_all_mixnode_reports()
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err, Status::InternalServerError))
}

#[get("/gateways/all/report")]
pub(crate) async fn gateways_full_report(
    storage: &State<NodeStatusStorage>,
) -> Result<Json<Vec<GatewayStatusReport>>, ErrorResponse> {
    storage
        .get_all_gateway_reports()
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err, Status::InternalServerError))
}
