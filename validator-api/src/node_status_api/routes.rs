// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    ErrorResponse, GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport,
    MixnodeUptimeHistory,
};
use crate::storage::NodeStatusStorage;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;

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
