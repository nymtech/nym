// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::helpers::{
    _compute_mixnode_reward_estimation, _gateway_core_status_count, _gateway_report,
    _gateway_uptime_history, _get_gateway_avg_uptime, _get_gateways_detailed,
    _get_gateways_detailed_unfiltered, _get_mixnode_avg_uptime, _get_mixnode_reward_estimation,
    _get_mixnodes_detailed_unfiltered, _mixnode_core_status_count, _mixnode_report,
    _mixnode_uptime_history,
};
use crate::node_status_api::models::AxumResult;
use crate::v2::AxumAppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use axum::Router;
use nym_api_requests::models::{
    ComputeRewardEstParam, GatewayBondAnnotated, GatewayCoreStatusResponse,
    GatewayStatusReportResponse, GatewayUptimeHistoryResponse, GatewayUptimeResponse,
    MixNodeBondAnnotated, MixnodeCoreStatusResponse, MixnodeStatusReportResponse,
    MixnodeUptimeHistoryResponse, RewardEstimationResponse, UptimeResponse,
};
use nym_mixnet_contract_common::MixId;
use serde::Deserialize;
use utoipa::IntoParams;

use super::unstable;

pub(super) fn network_monitor_routes() -> Router<AxumAppState> {
    Router::new().nest(
        "/gateway/:identity",
        Router::new()
            .route("/report", axum::routing::get(gateway_report))
            .route("/history", axum::routing::get(gateway_uptime_history))
            .route(
                "/core-status-count",
                axum::routing::get(gateway_core_status_count),
            )
            .route("/avg_uptime", axum::routing::get(get_gateway_avg_uptime))
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
            ),
    )
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateway/{identity}/report",
    responses(
        (status = 200, body = GatewayStatusReportResponse)
    )
)]
async fn gateway_report(
    Path(identity): Path<String>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<GatewayStatusReportResponse>> {
    Ok(Json(
        _gateway_report(state.node_status_cache(), &identity).await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateway/{identity}/history",
    responses(
        (status = 200, body = GatewayUptimeHistoryResponse)
    )
)]
async fn gateway_uptime_history(
    Path(identity): Path<String>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<GatewayUptimeHistoryResponse>> {
    Ok(Json(
        _gateway_uptime_history(state.storage(), &identity).await?,
    ))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SinceQueryParams {
    since: Option<i64>,
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    params(
        SinceQueryParams
    ),
    path = "/v1/status/gateway/{identity}/core-status-count",
    responses(
        (status = 200, body = GatewayCoreStatusResponse)
    )
)]
async fn gateway_core_status_count(
    Path(identity): Path<String>,
    Query(SinceQueryParams { since }): Query<SinceQueryParams>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<GatewayCoreStatusResponse>> {
    Ok(Json(
        _gateway_core_status_count(state.storage(), &identity, since).await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateway/{identity}/avg_uptime",
    responses(
        (status = 200, body = GatewayUptimeResponse)
    )
)]
async fn get_gateway_avg_uptime(
    Path(identity): Path<String>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<GatewayUptimeResponse>> {
    Ok(Json(
        _get_gateway_avg_uptime(state.node_status_cache(), &identity).await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/mixnode/{mix_id}/report",
    responses(
        (status = 200, body = MixnodeStatusReportResponse)
    )
)]
async fn mixnode_report(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<MixnodeStatusReportResponse>> {
    Ok(Json(
        _mixnode_report(state.node_status_cache(), mix_id).await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/mixnode/{mid_id}/history",
    responses(
        (status = 200, body = MixnodeUptimeHistoryResponse)
    )
)]
async fn mixnode_uptime_history(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<MixnodeUptimeHistoryResponse>> {
    Ok(Json(
        _mixnode_uptime_history(state.storage(), mix_id).await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    params(
        SinceQueryParams
    ),
    path = "/v1/status/mixnode/{mix_id}/core-status-count?since={since}",
    responses(
        (status = 200, body = MixnodeCoreStatusResponse)
    )
)]
async fn mixnode_core_status_count(
    Path(mix_id): Path<MixId>,
    Query(SinceQueryParams { since }): Query<SinceQueryParams>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<MixnodeCoreStatusResponse>> {
    Ok(Json(
        _mixnode_core_status_count(state.storage(), mix_id, since).await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/mixnode/{mix_id}/reward-estimation",
    responses(
        (status = 200, body = RewardEstimationResponse)
    )
)]
async fn get_mixnode_reward_estimation(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<RewardEstimationResponse>> {
    Ok(Json(
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
        ComputeRewardEstParam
    ),
    path = "/v1/status/mixnode/{mix_id}/compute-reward-estimation",
    request_body = ComputeRewardEstParam,
    responses(
        (status = 200, body = RewardEstimationResponse)
    )
)]
async fn compute_mixnode_reward_estimation(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
    Json(user_reward_param): Json<ComputeRewardEstParam>,
) -> AxumResult<Json<RewardEstimationResponse>> {
    Ok(Json(
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
    path = "/v1/status/mixnode/{mix_id}/avg_uptime",
    responses(
        (status = 200, body = UptimeResponse)
    )
)]
async fn get_mixnode_avg_uptime(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<UptimeResponse>> {
    Ok(Json(
        _get_mixnode_avg_uptime(state.node_status_cache(), mix_id).await?,
    ))
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/mixnodes/detailed-unfiltered",
    responses(
        (status = 200, body = MixNodeBondAnnotated)
    )
)]
pub async fn get_mixnodes_detailed_unfiltered(
    State(state): State<AxumAppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_mixnodes_detailed_unfiltered(state.node_status_cache()).await)
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateways/detailed",
    responses(
        (status = 200, body = GatewayBondAnnotated)
    )
)]
pub async fn get_gateways_detailed(
    State(state): State<AxumAppState>,
) -> Json<Vec<GatewayBondAnnotated>> {
    Json(_get_gateways_detailed(state.node_status_cache()).await)
}

#[utoipa::path(
    tag = "network-monitor-status",
    get,
    path = "/v1/status/gateways/detailed-unfiltered",
    responses(
        (status = 200, body = GatewayBondAnnotated)
    )
)]
pub async fn get_gateways_detailed_unfiltered(
    State(state): State<AxumAppState>,
) -> Json<Vec<GatewayBondAnnotated>> {
    Json(_get_gateways_detailed_unfiltered(state.node_status_cache()).await)
}
