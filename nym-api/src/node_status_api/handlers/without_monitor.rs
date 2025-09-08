// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::Json;
use axum::Router;
use nym_types::monitoring::MonitorMessage;
use tracing::error;

pub(super) fn mandatory_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/submit-gateway-monitoring-results",
            post(submit_gateway_monitoring_results),
        )
        .route(
            "/submit-node-monitoring-results",
            post(submit_node_monitoring_results),
        )
}

#[utoipa::path(
    tag = "status",
    post,
    path = "/v1/status/submit-gateway-monitoring-results",
    responses(
        (status = 200),
        (status = 400, body = String, description = "TBD"),
        (status = 403, body = String, description = "TBD"),
        (status = 500, body = String, description = "TBD"),
    ),
)]
pub(crate) async fn submit_gateway_monitoring_results(
    State(state): State<AppState>,
    Json(message): Json<MonitorMessage>,
) -> AxumResult<()> {
    if !message.is_in_allowed() {
        return Err(AxumErrorResponse::forbidden(
            "Monitor not registered to submit results",
        ));
    }

    if !message.timely() {
        return Err(AxumErrorResponse::bad_request("Message is too old"));
    }

    if !message.verify() {
        return Err(AxumErrorResponse::bad_request("invalid signature"));
    }

    match state
        .storage
        .submit_gateway_statuses_v2(message.results())
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("failed to submit gateway monitoring results: {err}");
            Err(AxumErrorResponse::internal_msg(
                "failed to submit gateway monitoring results",
            ))
        }
    }
}

#[utoipa::path(
    tag = "status",
    post,
    path = "/v1/status/submit-node-monitoring-results",
    responses(
        (status = 200),
        (status = 400, body = String, description = "TBD"),
        (status = 403, body = String, description = "TBD"),
        (status = 500, body = String, description = "TBD"),
    ),
)]
pub(crate) async fn submit_node_monitoring_results(
    State(state): State<AppState>,
    Json(message): Json<MonitorMessage>,
) -> AxumResult<()> {
    if !message.is_in_allowed() {
        return Err(AxumErrorResponse::forbidden(
            "Monitor not registered to submit results",
        ));
    }

    if !message.timely() {
        return Err(AxumErrorResponse::bad_request("Message is too old"));
    }

    if !message.verify() {
        return Err(AxumErrorResponse::bad_request("invalid signature"));
    }

    match state
        .storage
        .submit_mixnode_statuses_v2(message.results())
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("failed to submit node monitoring results: {err}");
            Err(AxumErrorResponse::internal_msg(
                "failed to submit node monitoring results",
            ))
        }
    }
}
