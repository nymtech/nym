// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::http::state::AppState;
use crate::orchestrator::prometheus::{
    AnnounceOutcome, PROMETHEUS_METRICS, PrometheusMetric, TestrunRequestOutcome,
};
use crate::storage::models::TestKind;
use axum::extract::{ConnectInfo, State};
use axum::routing::post;
use axum::{Json, Router};
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_network_monitor_orchestrator_requests::models::{
    AgentAnnounceRequest, AgentAnnounceResponse, OneHopMixnetResult, WorkAssignmentRequest,
    WorkAssignmentResponse, WorkResultSubmissionRequest, WorkResultSubmissionResponse,
};
use nym_network_monitor_orchestrator_requests::routes;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::ExecuteMsg as NetworkMonitorsExecuteMsg;
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
        (status = 400, description = "the announced addresses were not a plain ipv4/ipv6 pair"),
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

    // 1. make sure the agent announced one address of each family. anything else (a swapped pair,
    //    the same address twice, or an ipv4-mapped ipv6 address) would authorise addresses that
    //    don't match what the nodes will actually see, so it's rejected rather than normalised
    if !body.mix_addresses.has_distinct_families() {
        error!(
            "agent at {pod_ip} announced a malformed address pair: {} / {}",
            body.mix_addresses.v4, body.mix_addresses.v6
        );
        PROMETHEUS_METRICS.inc_for_announce_outcome(
            PrometheusMetric::AgentAnnounceOutcomeTotal,
            AnnounceOutcome::MalformedAddresses,
        );
        return Err(ApiError::MalformedAgentAddresses);
    }

    // 2. upsert the agent in the cache and learn whether it has already been announced
    let already_announced = state
        .agents
        .try_announce_agent(
            body.mix_addresses,
            body.x25519_noise_key,
            body.ed25519_identity,
        )
        .await;

    // 3. if the agent was already announced, skip the contract tx
    if already_announced {
        info!("agent at {pod_ip} is already announced, skipping contract tx");
        PROMETHEUS_METRICS.inc_for_announce_outcome(
            PrometheusMetric::AgentAnnounceOutcomeTotal,
            AnnounceOutcome::Duplicate,
        );
        return Ok(Json(AgentAnnounceResponse {}));
    }

    // 4. attempt to announce the agent to the network monitors contract.
    //    both of its addresses are authorised within a single transaction so that the agent is
    //    never left with only one of them authorised
    let msgs: Vec<(NetworkMonitorsExecuteMsg, Vec<Coin>)> =
        [body.mix_addresses.v4, body.mix_addresses.v6]
            .into_iter()
            .map(|mixnet_address| {
                (
                    NetworkMonitorsExecuteMsg::AuthoriseNetworkMonitor {
                        mixnet_address,
                        bs58_x25519_noise: body.x25519_noise_key.to_base58_string(),
                        noise_version: body.noise_version,
                        // both of the agent's entries carry the same identity: the gateway session
                        // gate is keyed on it alone, so it can't be tied to either address
                        bs58_ed25519_identity: Some(body.ed25519_identity.to_base58_string()),
                    },
                    Vec::new(),
                )
            })
            .collect();

    // we don't strictly need write access to the client, but we really don't want to send any
    // other transactions while this one is in flight as that would mess up the sequence numbers
    let client = state.validator_client.write().await;

    // SAFETY: the expect is fine as the orchestrator can't have started without the contract address
    #[allow(clippy::expect_used)]
    let contract_address = client
        .nyxd
        .network_monitors_contract_address()
        .expect("network monitors contract address is not available")
        .clone();

    client
        .nyxd
        .execute_multiple(
            &contract_address,
            msgs,
            Default::default(),
            format!(
                "authorising network monitor agent at {} and {}",
                body.mix_addresses.v4, body.mix_addresses.v6
            ),
        )
        .await
        .inspect_err(|err| {
            PROMETHEUS_METRICS.inc_for_announce_outcome(
                PrometheusMetric::AgentAnnounceOutcomeTotal,
                AnnounceOutcome::ContractFailure,
            );

            error!("failed to announce agent to the network monitors contract: {err}")
        })
        .map_err(|_| ApiError::ContractFailure)?;
    drop(client);

    PROMETHEUS_METRICS.inc_for_announce_outcome(
        PrometheusMetric::AgentAnnounceOutcomeTotal,
        AnnounceOutcome::ContractSuccess,
    );

    // 5. mark the agent as announced so subsequent calls are no-ops
    state.agents.mark_announced(body.mix_addresses).await;

    Ok(Json(AgentAnnounceResponse {}))
}

#[utoipa::path(
    operation_id = "v1_agent_request_work_assignment",
    tag = "Network Monitor Agent",
    post,
    request_body = WorkAssignmentRequest,
    path = "/request-work-assignment",
    context_path = "/v1/agent",
    security(("agents_token" = [])),
    responses(
        (status = 200, content(
            (WorkAssignmentResponse = "application/json"),
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
async fn request_work_assignment(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<WorkAssignmentRequest>,
) -> Result<Json<WorkAssignmentResponse>, ApiError> {
    let pod_ip = addr.ip();
    info!("received testrun request from pod at {pod_ip}");
    PROMETHEUS_METRICS.inc(PrometheusMetric::AgentTestrunRequests);

    // 1. ensure the agent still exists in our announced cache
    // in case there was a weird network failure between the calls
    let Some(agent) = state
        .agents
        .resolve_requesting_agent(body.mix_addresses)
        .await
    else {
        PROMETHEUS_METRICS.inc_for_request_outcome(
            PrometheusMetric::AgentTestrunRequestOutcomeTotal,
            TestrunRequestOutcome::UnknownAgent,
        );
        return Err(ApiError::AgentNotFound);
    };

    if !agent.announced {
        PROMETHEUS_METRICS.inc_for_request_outcome(
            PrometheusMetric::AgentTestrunRequestOutcomeTotal,
            TestrunRequestOutcome::NotAnnounced,
        );
        return Err(ApiError::AgentNotAnnounced);
    }

    // 2. hand out a due route run, else the most overdue one-hop kind
    let assignment = state.assign_work().await?;
    // a one-hop assignment is counted per kind where it is built, and a route run is not counted
    if assignment.is_none() {
        PROMETHEUS_METRICS.inc_for_request_outcome(
            PrometheusMetric::AgentTestrunRequestOutcomeTotal,
            TestrunRequestOutcome::NoWork,
        );
    }

    Ok(Json(WorkAssignmentResponse { assignment }))
}

fn emit_one_hop_mixnet_result_metrics(result: &OneHopMixnetResult) {
    PROMETHEUS_METRICS.inc(PrometheusMetric::TestRunResultSubmissions);

    // run-level: one observation per submitted run, against the kind the measurements were taken
    // for, since a liveness wave and a stress run are bounded by very different leases
    let kind: TestKind = result.result.measurements.kind().into();
    PROMETHEUS_METRICS.observe_for_kind(
        PrometheusMetric::TestDurationSeconds,
        kind,
        result.result.time_taken.as_secs_f64(),
    );
    if result.result.error.is_some() {
        PROMETHEUS_METRICS.inc(PrometheusMetric::TestrunsErrors)
    }

    // per-measurement: counters accumulate over every interface the run exercised, while the
    // histograms take one observation each, so a gateway run's two phases stay distinguishable in
    // the distribution rather than being averaged into one sample
    for measurement in result.result.measurements.all() {
        PROMETHEUS_METRICS.inc_by(
            PrometheusMetric::TestPacketsSent,
            measurement.packets_sent as i64,
        );
        PROMETHEUS_METRICS.inc_by(
            PrometheusMetric::TestPacketsReceived,
            measurement.packets_received as i64,
        );
        PROMETHEUS_METRICS.observe_histogram(
            PrometheusMetric::TestrunReceivedPacketsRatio,
            measurement.received_ratio(),
        );

        if let Some(latency) = measurement.approximate_latency {
            PROMETHEUS_METRICS.observe_histogram(
                PrometheusMetric::ApproximateNodeLatencyMs,
                latency.as_millis() as f64,
            );
        }

        if let Some(packets_stats) = measurement.packets_statistics {
            PROMETHEUS_METRICS.observe_histogram(
                PrometheusMetric::AverageTestPacketRTTMs,
                packets_stats.mean.as_millis() as f64,
            );
        }
    }
}

#[utoipa::path(
    operation_id = "v1_agent_submit_work_result",
    tag = "Network Monitor Agent",
    post,
    request_body = WorkResultSubmissionRequest,
    path = "/submit-work-result",
    context_path = "/v1/agent",
    security(("agents_token" = [])),
    responses(
        (status = 200, content(
            (WorkResultSubmissionResponse = "application/json"),
        )),
        (status = 400, description = "a one-hop result shaped for another kind than dispatched, or a route run submission with an unexpected or repeated gateway or a batch that is unreadable or names another gateway"),
        (status = 409, description = "the one-hop node's lease expired, or the route run is not in flight (unknown, already submitted or past its timeout)"),
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
async fn submit_work_result(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<WorkResultSubmissionRequest>,
) -> Result<Json<WorkResultSubmissionResponse>, ApiError> {
    let pod_ip = addr.ip();

    match body {
        WorkResultSubmissionRequest::OneHopMixnet(result) => {
            emit_one_hop_mixnet_result_metrics(&result);

            info!(
                "received testrun result for node {} at {} from pod at {pod_ip}",
                result.node_id, result.tested_address
            );

            state
                .submit_one_hop_mixnet_result(result.result, result.node_id, result.tested_address)
                .await?;
        }
        WorkResultSubmissionRequest::FiveHopMixnet(result) => {
            info!(
                "received route run {} result with {} sessions from pod at {pod_ip}",
                result.run_id,
                result.sessions.len()
            );

            state.submit_five_hop_mixnet_result(result).await?;
        }
    }

    Ok(Json(WorkResultSubmissionResponse {}))
}

/// Builds the agent sub-router with all agent endpoints behind bearer-token auth.
pub(super) fn routes(auth_layer: AuthLayer) -> Router<AppState> {
    Router::new()
        .route(routes::v1::agent::ANNOUNCE, post(announce_agent))
        .route(
            routes::v1::agent::REQUEST_WORK_ASSIGNMENT,
            post(request_work_assignment),
        )
        .route(
            routes::v1::agent::SUBMIT_WORK_RESULT,
            post(submit_work_result),
        )
        .route_layer(auth_layer)
}
