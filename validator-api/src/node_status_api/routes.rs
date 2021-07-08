// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{ErrorResponse, GatewayStatusReport, MixnodeStatusReport};
use crate::node_status_api::storage::NodeStatusStorage;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;

#[get("/mixnode/<pubkey>/report")]
pub(crate) async fn mixnode_report(
    storage: &State<NodeStatusStorage>,
    pubkey: &str,
) -> Result<Json<MixnodeStatusReport>, ErrorResponse> {
    storage
        .get_mixnode_report(pubkey)
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
        .get_gateway_report(pubkey)
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
