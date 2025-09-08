// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::unstable;
use crate::node_status_api::handlers::unstable::{latest_monitor_run_report, monitor_run_report};
use crate::node_status_api::handlers::MixIdParam;
use crate::node_status_api::helpers::{
    _gateway_core_status_count, _gateway_uptime_history, _mixnode_core_status_count,
    _mixnode_uptime_history,
};
use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Router;
use nym_api_requests::models::{
    GatewayCoreStatusResponse, GatewayUptimeHistoryResponse, MixnodeCoreStatusResponse,
    MixnodeUptimeHistoryResponse,
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
                .route("/history", axum::routing::get(gateway_uptime_history))
                .route(
                    "/core-status-count",
                    axum::routing::get(gateway_core_status_count),
                ),
        )
        .nest(
            "/mixnode/:mix_id",
            Router::new()
                .route("/history", axum::routing::get(mixnode_uptime_history))
                .route(
                    "/core-status-count",
                    axum::routing::get(mixnode_core_status_count),
                ),
        )
        .nest(
            "/mixnodes",
            Router::new().route(
                "/unstable/:mix_id/test-results",
                axum::routing::get(unstable::mixnode_test_results),
            ),
        )
        .nest(
            "/gateways",
            Router::new().route(
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

    Ok(output.to_response(_gateway_uptime_history(state.storage(), &identity).await?))
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

    Ok(output.to_response(_mixnode_uptime_history(state.storage(), mix_id).await?))
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
