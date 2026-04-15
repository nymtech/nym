// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::{AppState, KnownAgents};
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_network_monitor_orchestrator_requests::models::{
    AgentAnnounceRequest, AgentPortRequest, AgentPortRequestResponse, TestRunAssignment,
};
use nym_network_monitor_orchestrator_requests::routes;
use nym_validator_client::nyxd::contract_traits::NetworkMonitorsSigningClient;
use std::net::SocketAddr;
use tracing::{error, info};

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
    State(agents): State<KnownAgents>,
    Json(body): Json<AgentPortRequest>,
) -> impl IntoResponse {
    let pod_ip = addr.ip();
    info!("received port request from pod at {pod_ip}: {body:?}");
    let available_mix_port = agents
        .assign_agent_port(body.agent_node_ip, body.x25519_noise_key)
        .await;
    info!("assigned port {available_mix_port} to agent at {pod_ip}");

    Json(AgentPortRequestResponse { available_mix_port })
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

    // 1. if the agent does not exist in the cache - reject it,
    // there's some data inconsistency in the system,
    // orchestrator might have restarted between agent requesting port
    // and sending the announce request
    // in that case, it should just try the whole procedure again
    if !state
        .agents
        .touch_agent(body.agent_mix_socket_address)
        .await
    {
        return (StatusCode::BAD_REQUEST, "agent information not found").into_response();
    }

    // 2. attempt to announce the agent to the network monitors contract
    if let Err(err) = state
        .validator_client
        .write()
        .await
        .nyxd
        .authorise_network_monitor(
            body.agent_mix_socket_address,
            body.x25519_noise_key.to_base58_string(),
            body.noise_version,
            None,
        )
        .await
    {
        error!("failed to announce agent to the network monitors contract: {err}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to announce agent to the network monitors contract:",
        )
            .into_response();
    }

    Json(()).into_response()
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

    error!("unimplemented");

    Json(TestRunAssignment {
        assignment: Some(()),
    })
}

/// Builds the agent sub-router with all agent endpoints behind bearer-token auth.
pub(super) fn routes(auth_layer: AuthLayer) -> Router<AppState> {
    Router::new()
        .route(routes::v1::agent::PORT_REQUEST, post(request_mix_port))
        .route(routes::v1::agent::ANNOUNCE, post(announce_agent))
        .route(routes::v1::agent::REQUEST_TESTRUN, get(request_testrun))
        .route_layer(auth_layer)
}
