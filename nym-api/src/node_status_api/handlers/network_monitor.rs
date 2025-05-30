// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::unstable;
use crate::node_status_api::handlers::unstable::{latest_monitor_run_report, monitor_run_report};
use crate::node_status_api::handlers::MixIdParam;
use crate::node_status_api::helpers::{
    _compute_mixnode_reward_estimation, _gateway_core_status_count, _gateway_report,
    _gateway_uptime_history, _get_gateway_avg_uptime, _get_legacy_gateways_detailed,
    _get_legacy_gateways_detailed_unfiltered, _get_mixnode_avg_uptime,
    _get_mixnode_reward_estimation, _get_mixnodes_detailed_unfiltered, _mixnode_core_status_count,
    _mixnode_report, _mixnode_uptime_history,
};
use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use axum::Router;
use nym_api_requests::models::{
    ComputeRewardEstParam, GatewayBondAnnotated, GatewayCoreStatusResponse,
    GatewayStatusReportResponse, GatewayUptimeHistoryResponse, GatewayUptimeResponse,
    MixNodeBondAnnotated, MixnodeCoreStatusResponse, MixnodeStatusReportResponse,
    MixnodeUptimeHistoryResponse, RewardEstimationResponse, UptimeResponse,
};
use nym_http_api_common::{FormattedResponse, Output, OutputParams};
use serde::Deserialize;
use utoipa::IntoParams;

// we want to mark the routes as deprecated in swagger, but still expose them
#[allow(deprecated)]
pub(super) fn network_monitor_routes() -> Router<AppState> {
    Router::new()
        .nest(
            "/gateway/:identity",
            Router::new()
                .route("/report", axum::routing::get(gateway_report))
                .route("/history", axum::routing::get(gateway_uptime_history))
                .route(
                    "/core-status-count",
                    axum::routing::get(gateway_core_status_count),
                )
                .route("/avg_uptime", axum::routing::get(get_gateway_avg_uptime)),
        )
        .nest(
            "/mixnode/:mix_id",
            Router::new()
                .route("/report", axum::routing::get(mixnode_report))
                .route("/history", axum::routing::get(mixnode_uptime_history))
                .route(
                    "/core-status-count",
                    axum::routing::get(mixnode_core_status_count),
                )
                .route(
                    "/reward-estimation",
                    axum::routing::get(get_mixnode_reward_estimation),
                )
                .route(
                    "/compute-reward-estimation",
                    axum::routing::post(compute_mixnode_reward_estimation),
                )
                .route("/avg_uptime", axum::routing::get(get_mixnode_avg_uptime)),
        )
        .nest(
            "/mixnodes",
            Router::new()
                .route(
                    "/detailed-unfiltered",
                    axum::routing::get(get_mixnodes_detailed_unfiltered),
                )
                .route(
                    "/unstable/:mix_id/test-results",
                    axum::routing::get(unstable::mixnode_test_results),
                ),
        )
        .nest(
            "/gateways",
            Router::new()
                .route("/detailed", axum::routing::get(get_gateways_detailed))
                .route(
                    "/detailed-unfiltered",
                    axum::routing::get(get_gateways_detailed_unfiltered),
                )
                .route(
                    "/unstable/:gateway_identity/test-results",
                    axum::routing::get(unstable::gateway_test_results),
                ),
        )
        .nest(
            "/network-monitor/unstable",
            Router::new()
                .route(
                    "/run/:monitor_run_id/details",
                    axum::routing::get(monitor_run_report),
                )
                .route(
                    "/run/latest/details",
                    axum::routing::get(latest_monitor_run_report),
                ),
        )
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateway/{identity}/report",
    responses(
        (status = 200, content(
            (GatewayStatusReportResponse = "application/json"),
            (GatewayStatusReportResponse = "application/yaml"),
            (GatewayStatusReportResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn gateway_report(
    Path(identity): Path<String>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<GatewayStatusReportResponse>> {
    let output = output.output.unwrap_or_default();

    Ok(output.to_response(_gateway_report(state.node_status_cache(), &identity).await?))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateway/{identity}/history",
    responses(
        (status = 200, content(
            (GatewayUptimeHistoryResponse = "application/json"),
            (GatewayUptimeHistoryResponse = "application/yaml"),
            (GatewayUptimeHistoryResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn gateway_uptime_history(
    Path(identity): Path<String>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<GatewayUptimeHistoryResponse>> {
    let output = output.output.unwrap_or_default();

    Ok(output.to_response(
        _gateway_uptime_history(state.storage(), state.nym_contract_cache(), &identity).await?,
    ))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SinceQueryParams {
    since: Option<i64>,
    output: Option<Output>,
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    params(
        SinceQueryParams
    ),
    path = "/v1/status/gateway/{identity}/core-status-count",
    responses(
        (status = 200, content(
            (GatewayCoreStatusResponse = "application/json"),
            (GatewayCoreStatusResponse = "application/yaml"),
            (GatewayCoreStatusResponse = "application/bincode")
        ))
    ),
)]
#[deprecated]
async fn gateway_core_status_count(
    Path(identity): Path<String>,
    Query(SinceQueryParams { since, output }): Query<SinceQueryParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<GatewayCoreStatusResponse>> {
    Ok(output
        .unwrap_or_default()
        .to_response(_gateway_core_status_count(state.storage(), &identity, since).await?))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateway/{identity}/avg_uptime",
    responses(
        (status = 200, content(
            (GatewayUptimeResponse = "application/json"),
            (GatewayUptimeResponse = "application/yaml"),
            (GatewayUptimeResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_gateway_avg_uptime(
    Path(identity): Path<String>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<GatewayUptimeResponse>> {
    let output = output.output.unwrap_or_default();

    Ok(output.to_response(_get_gateway_avg_uptime(state.node_status_cache(), &identity).await?))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/mixnode/{mix_id}/report",
    responses(
        (status = 200, content(
            (MixnodeStatusReportResponse = "application/json"),
            (MixnodeStatusReportResponse = "application/yaml"),
            (MixnodeStatusReportResponse = "application/bincode")
        ))
    ),
    params(OutputParams, MixIdParam)
)]
#[deprecated]
async fn mixnode_report(
    Path(MixIdParam { mix_id }): Path<MixIdParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<MixnodeStatusReportResponse>> {
    let output = output.output.unwrap_or_default();

    Ok(output.to_response(_mixnode_report(state.node_status_cache(), mix_id).await?))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/mixnode/{mix_id}/history",
    responses(
        (status = 200, content(
            (MixnodeUptimeHistoryResponse = "application/json"),
            (MixnodeUptimeHistoryResponse = "application/yaml"),
            (MixnodeUptimeHistoryResponse = "application/bincode")
        ))
    ),
    params(MixIdParam, OutputParams)
)]
#[deprecated]
async fn mixnode_uptime_history(
    Path(MixIdParam { mix_id }): Path<MixIdParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<MixnodeUptimeHistoryResponse>> {
    let output = output.output.unwrap_or_default();

    Ok(output.to_response(
        _mixnode_uptime_history(state.storage(), state.nym_contract_cache(), mix_id).await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    params(
        MixIdParam, SinceQueryParams
    ),
    path = "/v1/status/mixnode/{mix_id}/core-status-count",
    responses(
        (status = 200, content(
            (MixnodeCoreStatusResponse = "application/json"),
            (MixnodeCoreStatusResponse = "application/yaml"),
            (MixnodeCoreStatusResponse = "application/bincode")
        ))
    ),
)]
#[deprecated]
async fn mixnode_core_status_count(
    Path(MixIdParam { mix_id }): Path<MixIdParam>,
    Query(SinceQueryParams { since, output }): Query<SinceQueryParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<MixnodeCoreStatusResponse>> {
    let output = output.unwrap_or_default();

    Ok(output.to_response(_mixnode_core_status_count(state.storage(), mix_id, since).await?))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    params(
        MixIdParam, OutputParams
    ),
    path = "/v1/status/mixnode/{mix_id}/reward-estimation",
    responses(
        (status = 200, content(
            (RewardEstimationResponse = "application/json"),
            (RewardEstimationResponse = "application/yaml"),
            (RewardEstimationResponse = "application/bincode")
        ))
    ),
)]
#[deprecated]
async fn get_mixnode_reward_estimation(
    Path(MixIdParam { mix_id }): Path<MixIdParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<RewardEstimationResponse>> {
    let output = output.output.unwrap_or_default();

    Ok(output.to_response(
        _get_mixnode_reward_estimation(
            state.node_status_cache(),
            state.nym_contract_cache(),
            mix_id,
        )
        .await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    post,
    params(
        OutputParams, MixIdParam
    ),
    path = "/v1/status/mixnode/{mix_id}/compute-reward-estimation",
    request_body = ComputeRewardEstParam,
    responses(
        (status = 200, content(
            (RewardEstimationResponse = "application/json"),
            (RewardEstimationResponse = "application/yaml"),
            (RewardEstimationResponse = "application/bincode")
        ))
    ),
)]
#[deprecated]
async fn compute_mixnode_reward_estimation(
    Path(MixIdParam { mix_id }): Path<MixIdParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
    Json(user_reward_param): Json<ComputeRewardEstParam>,
) -> AxumResult<FormattedResponse<RewardEstimationResponse>> {
    let output = output.output.unwrap_or_default();

    Ok(output.to_response(
        _compute_mixnode_reward_estimation(
            &user_reward_param,
            state.node_status_cache(),
            state.nym_contract_cache(),
            mix_id,
        )
        .await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    params(
        MixIdParam, OutputParams
    ),
    path = "/v1/status/mixnode/{mix_id}/avg_uptime",
    responses(
        (status = 200, content(
            (UptimeResponse = "application/json"),
            (UptimeResponse = "application/yaml"),
            (UptimeResponse = "application/bincode")
        ))
    ),
)]
#[deprecated]
async fn get_mixnode_avg_uptime(
    Path(MixIdParam { mix_id }): Path<MixIdParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<UptimeResponse>> {
    let output = output.output.unwrap_or_default();

    Ok(output.to_response(_get_mixnode_avg_uptime(state.node_status_cache(), mix_id).await?))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/mixnodes/detailed-unfiltered",
    responses(
        (status = 200, content(
            (MixNodeBondAnnotated = "application/json"),
            (MixNodeBondAnnotated = "application/yaml"),
            (MixNodeBondAnnotated = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
pub async fn get_mixnodes_detailed_unfiltered(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<MixNodeBondAnnotated>> {
    let output = output.output.unwrap_or_default();

    output.to_response(_get_mixnodes_detailed_unfiltered(state.node_status_cache()).await)
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateways/detailed",
    responses(
        (status = 200, content(
            (GatewayBondAnnotated = "application/json"),
            (GatewayBondAnnotated = "application/yaml"),
            (GatewayBondAnnotated = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
pub async fn get_gateways_detailed(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<GatewayBondAnnotated>> {
    let output = output.output.unwrap_or_default();

    output.to_response(_get_legacy_gateways_detailed(state.node_status_cache()).await)
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateways/detailed-unfiltered",
    responses(
        (status = 200, content(
            (GatewayBondAnnotated = "application/json"),
            (GatewayBondAnnotated = "application/yaml"),
            (GatewayBondAnnotated = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
pub async fn get_gateways_detailed_unfiltered(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<GatewayBondAnnotated>> {
    let output = output.output.unwrap_or_default();

    output.to_response(_get_legacy_gateways_detailed_unfiltered(state.node_status_cache()).await)
}
