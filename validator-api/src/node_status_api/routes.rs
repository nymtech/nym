// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{ErrorResponseNew, GatewayStatusReport, MixnodeStatusReport};
use crate::node_status_api::storage::NodeStatusStorage;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::State;

#[get("/mixnode/<pubkey>/report")]
pub(crate) async fn mixnode_report(
    storage: &State<NodeStatusStorage>,
    pubkey: &str,
) -> Result<Json<MixnodeStatusReport>, ErrorResponseNew> {
    storage
        .get_mixnode_report(pubkey)
        .await
        .map(Json)
        .map_err(|err| ErrorResponseNew::new(err, Status::NotFound))
}

#[get("/gateway/<pubkey>/report")]
pub(crate) async fn gateway_report(
    storage: &State<NodeStatusStorage>,
    pubkey: &str,
) -> Result<Json<GatewayStatusReport>, ErrorResponseNew> {
    storage
        .get_gateway_report(pubkey)
        .await
        .map(Json)
        .map_err(|err| ErrorResponseNew::new(err, Status::NotFound))
}

#[get("/mixnodes/all/report")]
pub(crate) async fn mixnodes_full_report(
    storage: &State<NodeStatusStorage>,
) -> Result<Json<Vec<MixnodeStatusReport>>, ErrorResponseNew> {
    storage
        .get_all_mixnode_reports()
        .await
        .map(Json)
        .map_err(|err| ErrorResponseNew::new(err, Status::InternalServerError))
}

#[get("/gateways/all/report")]
pub(crate) async fn gateways_full_report(
    storage: &State<NodeStatusStorage>,
) -> Result<Json<Vec<GatewayStatusReport>>, ErrorResponseNew> {
    storage
        .get_all_gateway_reports()
        .await
        .map(Json)
        .map_err(|err| ErrorResponseNew::new(err, Status::InternalServerError))
}
