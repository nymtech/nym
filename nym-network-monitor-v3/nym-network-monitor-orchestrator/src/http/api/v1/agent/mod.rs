// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::http::state::{AppState, KnownAgents};
use axum::extract::{ConnectInfo, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_network_monitor_orchestrator_requests::models::{
    AgentAnnounceRequest, AgentAnnounceResponse, AgentPortRequest, AgentPortRequestResponse,
    TestRunAssignment,
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
) -> Result<Json<AgentPortRequestResponse>, ApiError> {
    let pod_ip = addr.ip();
    info!("received port request from pod at {pod_ip}: {body:?}");

    let available_mix_port = agents
        .assign_agent_port(body.agent_node_ip, body.x25519_noise_key)
        .await
        .ok_or_else(|| {
            error!("no available ports for agent at {pod_ip}");
            ApiError::NoPortsAvailable
        })?;

    info!("assigned port {available_mix_port} to agent at {pod_ip}");
    Ok(Json(AgentPortRequestResponse { available_mix_port }))
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
) -> Result<Json<AgentAnnounceResponse>, ApiError> {
    let pod_ip = addr.ip();
    info!("received announce request from pod at {pod_ip}: {body:?}");

    // 1. validate the agent exists in the cache and the noise key matches
    let already_announced = state
        .agents
        .try_announce_agent(body.agent_mix_socket_address, body.x25519_noise_key)
        .await?;

    // 2. if the agent was already announced, skip the contract tx
    if already_announced {
        info!("agent at {pod_ip} is already announced, skipping contract tx");
        return Ok(Json(AgentAnnounceResponse {}));
    }

    // 3. attempt to announce the agent to the network monitors contract
    state
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
        .inspect_err(|err| {
            error!("failed to announce agent to the network monitors contract: {err}")
        })
        .map_err(|_| ApiError::ContractFailure)?;

    // 4. mark the agent as announced so subsequent calls are no-ops
    state
        .agents
        .mark_announced(body.agent_mix_socket_address)
        .await;

    Ok(Json(AgentAnnounceResponse {}))
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
async fn request_testrun(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<TestRunAssignment>, ApiError> {
    let pod_ip = addr.ip();

    info!("received testrun request from pod at {pod_ip}");

    error!("unimplemented");

    Ok(Json(TestRunAssignment {
        assignment: Some(()),
    }))
}

/// Builds the agent sub-router with all agent endpoints behind bearer-token auth.
pub(super) fn routes(auth_layer: AuthLayer) -> Router<AppState> {
    Router::new()
        .route(routes::v1::agent::PORT_REQUEST, post(request_mix_port))
        .route(routes::v1::agent::ANNOUNCE, post(announce_agent))
        .route(routes::v1::agent::REQUEST_TESTRUN, get(request_testrun))
        .route_layer(auth_layer)
}
