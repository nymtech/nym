// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AppState;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_network_monitor_orchestrator_requests::models::{
    AgentAnnounceRequest, AgentPortRequest, AgentPortRequestResponse,
};
use nym_network_monitor_orchestrator_requests::routes;
use std::net::SocketAddr;
use tracing::info;

#[utoipa::path(
    operation_id = "v1_agent_port_request",
    tag = "Network Monitor Agent",
    post,
    request_body = AgentPortRequest,
    path = "/port-request",
    context_path = "/v1/agent",
    responses(
        (status = 200, content(
            (AgentPortRequestResponse = "application/json"),
        ))
    )
)]
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(
        agent_pod = %addr
    )
)]
async fn request_mix_port(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<AgentPortRequest>,
) -> impl IntoResponse {
    let pod_ip = addr.ip();
    info!("received port request from pod at {pod_ip}: {body:?}");

    let _ = state;

    Json(AgentPortRequestResponse {
        available_mix_port: 1234,
    })
}

#[utoipa::path(
    operation_id = "v1_agent_announce",
    tag = "Network Monitor Agent",
    post,
    request_body = AgentAnnounceRequest,
    path = "/announce",
    context_path = "/v1/agent",
    responses(
        (status = 200, content(

        ))
    )
)]
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(
        agent_pod = %addr
    )
)]
async fn announce_agent(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<AgentAnnounceRequest>,
) -> impl IntoResponse {
    let pod_ip = addr.ip();
    info!("received announce request from pod at {pod_ip}: {body:?}");

    // TODO: call the contract here
    let _ = state;

    StatusCode::OK
}

#[utoipa::path(
    operation_id = "v1_agent_request_testrun",
    tag = "Network Monitor Agent",
    get,
    path = "/request-testrun",
    context_path = "/v1/agent",
    responses(
        (status = 200, content(

        ))
    )
)]
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(
        agent_pod = %addr
    )
)]
async fn request_testrun(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    let pod_ip = addr.ip();

    info!("received testrun request from pod at {pod_ip}");

    StatusCode::OK
}

/// Builds the agent sub-router with all agent endpoints behind bearer-token auth.
pub(super) fn routes(auth_layer: AuthLayer) -> Router<AppState> {
    Router::new()
        .route(routes::v1::agent::PORT_REQUEST, post(request_mix_port))
        .route(routes::v1::agent::ANNOUNCE, post(announce_agent))
        .route(routes::v1::agent::REQUEST_TESTRUN, get(request_testrun))
        .route_layer(auth_layer)
}
