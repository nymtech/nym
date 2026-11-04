// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::support::http::helpers::NodeIdParam;
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::performance::NodePerformanceResponseV3;
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use nym_performance_contract_common::EpochWeights;

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    operation_id = "v3_get_node_performance",
    path = "/performance/{node_id}",
    context_path = "/v3/nym-nodes",
    responses(
        (status = 200, content(
            (NodePerformanceResponseV3 = "application/json"),
            (NodePerformanceResponseV3 = "application/yaml"),
        )),
        (status = 503, description = "no performance snapshot has settled yet"),
    ),
    params(NodeIdParam, OutputParamsV2),
)]
async fn get_node_performance(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<NodePerformanceResponseV3>> {
    let performance = state.latest_snapshot().await?.node_performance(node_id);
    Ok(output.to_response(performance))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/performance-weights",
    context_path = "/v3/nym-nodes",
    responses(
        (status = 200, content(
            (EpochWeights = "application/json"),
            (EpochWeights = "application/yaml"),
        )),
        (status = 503, description = "no performance snapshot has settled yet"),
    ),
    params(OutputParamsV2),
)]
async fn get_performance_weights(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<EpochWeights>> {
    let weights = state
        .latest_snapshot()
        .await?
        .performance_auxiliary
        .weights();
    Ok(output.to_response(weights))
}

/// Build the `/v3/nym-nodes` subtree hosting the per-node performance read from the latest
/// settled performance snapshot.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/performance/{node_id}", get(get_node_performance))
        .route("/performance-weights", get(get_performance_weights))
}
