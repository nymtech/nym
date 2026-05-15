// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::http::state::AppState;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use axum::extract::{ConnectInfo, State};
use axum::routing::post;
use axum::{Json, Router};
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_network_monitor_orchestrator_requests::models::{
    AgentAnnounceRequest, AgentAnnounceResponse, TestRunAssignmentRequest,
    TestRunAssignmentResponse, TestRunResultSubmissionRequest, TestRunSubmissionResponse,
};
use nym_network_monitor_orchestrator_requests::routes;
use nym_validator_client::nyxd::contract_traits::NetworkMonitorsSigningClient;
use std::net::SocketAddr;
use tracing::{error, info};

#[utoipa::path(
    operation_id = "v1_agent_announce",
    tag = "Network Monitor Agent",
    post,
    request_body = AgentAnnounceRequest,
    path = "/announce",
    context_path = "/v1/agent",
    security(("agents_token" = [])),
    responses(
        (status = 200, content(
            (AgentAnnounceResponse = "application/json"),
        )),
        (status = 500, description = "failed to announce agent to the network monitors contract"),
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

    PROMETHEUS_METRICS.inc(PrometheusMetric::AgentAnnounceRequests);

    // 1. upsert the agent in the cache and learn whether it has already been announced
    let already_announced = state
        .agents
        .try_announce_agent(body.agent_mix_socket_address, body.x25519_noise_key)
        .await;

    // 2. if the agent was already announced, skip the contract tx
    if already_announced {
        info!("agent at {pod_ip} is already announced, skipping contract tx");
        PROMETHEUS_METRICS.inc(PrometheusMetric::AgentDuplicateAnnouncementRequests);
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
            PROMETHEUS_METRICS.inc(PrometheusMetric::AgentContractAnnounceFailures);

            error!("failed to announce agent to the network monitors contract: {err}")
        })
        .map_err(|_| ApiError::ContractFailure)?;

    PROMETHEUS_METRICS.inc(PrometheusMetric::AgentContractAnnounceSuccesses);

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
    post,
    request_body = TestRunAssignmentRequest,
    path = "/request-testrun",
    context_path = "/v1/agent",
    security(("agents_token" = [])),
    responses(
        (status = 200, content(
            (TestRunAssignmentResponse = "application/json"),
        )),
        (status = 400, description = "agent not found in cache, or agent has not yet been announced to the contract"),
        (status = 500, description = "failed to read from storage, or a stored field could not be decoded"),
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
    State(state): State<AppState>,
    Json(body): Json<TestRunAssignmentRequest>,
) -> Result<Json<TestRunAssignmentResponse>, ApiError> {
    let pod_ip = addr.ip();
    info!("received testrun request from pod at {pod_ip}");
    PROMETHEUS_METRICS.inc(PrometheusMetric::AgentTestrunRequests);

    // 1. ensure the agent still exists in our announced cache
    // in case there was a weird network failure between the calls
    let Some(agent) = state.agents.get_agent(body.agent_mix_socket_address).await else {
        PROMETHEUS_METRICS.inc(PrometheusMetric::AgentUnknownAgentTestrunRequests);
        return Err(ApiError::AgentNotFound);
    };

    if !agent.announced {
        PROMETHEUS_METRICS.inc(PrometheusMetric::AgentTestrunRequestsWithoutAnnouncement);
        return Err(ApiError::AgentNotAnnounced);
    }

    // 2. attempt to assign a testrun to the agent
    let assignment = state.assign_next_mixnode_testrun().await?;
    if assignment.is_none() {
        PROMETHEUS_METRICS.inc(PrometheusMetric::EmptyTestrunAssignments);
    } else {
        PROMETHEUS_METRICS.inc(PrometheusMetric::NonEmptyTestrunAssignments);
    }

    Ok(Json(TestRunAssignmentResponse { assignment }))
}

fn emit_testrun_result_metrics(result: &TestRunResultSubmissionRequest) {
    PROMETHEUS_METRICS.inc(PrometheusMetric::TestRunResultSubmissions);

    PROMETHEUS_METRICS.inc_by(
        PrometheusMetric::TestPacketsSent,
        result.result.packets_sent as i64,
    );
    PROMETHEUS_METRICS.inc_by(
        PrometheusMetric::TestPacketsReceived,
        result.result.packets_received as i64,
    );

    PROMETHEUS_METRICS.observe_histogram(
        PrometheusMetric::TestDurationSeconds,
        result.result.time_taken.as_secs_f64(),
    );
    if let Some(latency) = result.result.approximate_latency {
        PROMETHEUS_METRICS.observe_histogram(
            PrometheusMetric::ApproximateNodeLatencyMs,
            latency.as_millis() as f64,
        );
    }
    PROMETHEUS_METRICS.observe_histogram(
        PrometheusMetric::TestrunReceivedPacketsRatio,
        result.result.received_ratio(),
    );

    if let Some(packets_stats) = result.result.packets_statistics {
        PROMETHEUS_METRICS.observe_histogram(
            PrometheusMetric::AverageTestPacketRTTMs,
            packets_stats.mean.as_millis() as f64,
        );
    }

    if result.result.error.is_some() {
        PROMETHEUS_METRICS.inc(PrometheusMetric::TestrunsErrors)
    }
}

#[utoipa::path(
    operation_id = "v1_agent_submit_testrun_result",
    tag = "Network Monitor Agent",
    post,
    request_body = TestRunResultSubmissionRequest,
    path = "/submit-testrun-result",
    context_path = "/v1/agent",
    security(("agents_token" = [])),
    responses(
        (status = 200, content(
            (TestRunSubmissionResponse = "application/json"),
        )),
        (status = 500, description = "failed to persist the test run result to storage"),
    )
)]
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(
        agent_pod = %addr
    )
)]
async fn submit_testrun_result(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<TestRunResultSubmissionRequest>,
) -> Result<Json<TestRunSubmissionResponse>, ApiError> {
    let pod_ip = addr.ip();

    emit_testrun_result_metrics(&body);

    info!(
        "received testrun result for node {} from pod at {pod_ip}",
        body.node_id
    );

    state
        .submit_testrun_result(body.result, body.node_id)
        .await?;

    Ok(Json(TestRunSubmissionResponse {}))
}

/// Builds the agent sub-router with all agent endpoints behind bearer-token auth.
pub(super) fn routes(auth_layer: AuthLayer) -> Router<AppState> {
    Router::new()
        .route(routes::v1::agent::ANNOUNCE, post(announce_agent))
        .route(routes::v1::agent::REQUEST_TESTRUN, post(request_testrun))
        .route(
            routes::v1::agent::SUBMIT_TESTRUN_RESULT,
            post(submit_testrun_result),
        )
        .route_layer(auth_layer)
}
