// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::http::state::AppState;
use axum::extract::{Path, Query};
use axum::routing::get;
use axum::{Json, Router};
use nym_network_monitor_orchestrator_requests::models::{
    NymNodeData, PagedResult, Pagination, TestRunData, TestRunsInProgressResponse,
};
use nym_network_monitor_orchestrator_requests::routes;
use nym_validator_client::client::NodeId;

#[utoipa::path(
    operation_id = "v1_results_testrun_by_id",
    tag = "Network Monitor Results",
    get,
    params(("id" = i64, Path, description = "Database-assigned test-run id")),
    path = "/testrun/{id}",
    context_path = "/v1/results",
    security(("metrics_and_results_token" = [])),
    responses(
        (status = 200, content(
            (TestRunData = "application/json"),
        ))
    )
)]
async fn get_testrun_by_id(Path(_id): Path<i64>) -> Result<Json<TestRunData>, ApiError> {
    Err(ApiError::Unimplemented)
}

#[utoipa::path(
    operation_id = "v1_results_nym_node_by_node_id",
    tag = "Network Monitor Results",
    get,
    params(("node_id" = u32, Path, description = "Mixnet-contract node id")),
    path = "/nym-node/{node_id}",
    context_path = "/v1/results",
    security(("metrics_and_results_token" = [])),
    responses(
        (status = 200, content(
            (NymNodeData = "application/json"),
        ))
    )
)]
async fn get_nym_node_by_id(Path(_node_id): Path<NodeId>) -> Result<Json<NymNodeData>, ApiError> {
    Err(ApiError::Unimplemented)
}

#[utoipa::path(
    operation_id = "v1_results_testruns_in_progress",
    tag = "Network Monitor Results",
    get,
    path = "/testruns-in-progress",
    context_path = "/v1/results",
    security(("metrics_and_results_token" = [])),
    responses(
        (status = 200, content(
            (TestRunsInProgressResponse = "application/json"),
        ))
    )
)]
async fn get_testruns_in_progress() -> Result<Json<TestRunsInProgressResponse>, ApiError> {
    Err(ApiError::Unimplemented)
}

#[utoipa::path(
    operation_id = "v1_results_testruns",
    tag = "Network Monitor Results",
    get,
    params(Pagination),
    path = "/testruns",
    context_path = "/v1/results",
    security(("metrics_and_results_token" = [])),
    responses(
        (status = 200, content(
            (PagedResult<TestRunData> = "application/json"),
        ))
    )
)]
async fn get_testruns(
    Query(_pagination): Query<Pagination>,
) -> Result<Json<PagedResult<TestRunData>>, ApiError> {
    Err(ApiError::Unimplemented)
}

#[utoipa::path(
    operation_id = "v1_results_nym_nodes",
    tag = "Network Monitor Results",
    get,
    params(Pagination),
    path = "/nym-nodes",
    context_path = "/v1/results",
    security(("metrics_and_results_token" = [])),
    responses(
        (status = 200, content(
            (PagedResult<NymNodeData> = "application/json"),
        ))
    )
)]
async fn get_nym_nodes(
    Query(_pagination): Query<Pagination>,
) -> Result<Json<PagedResult<NymNodeData>>, ApiError> {
    Err(ApiError::Unimplemented)
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(routes::v1::results::TESTRUN_BY_ID, get(get_testrun_by_id))
        .route(
            routes::v1::results::NYM_NODE_BY_NODE_ID,
            get(get_nym_node_by_id),
        )
        .route(
            routes::v1::results::TESTRUNS_IN_PROGRESS,
            get(get_testruns_in_progress),
        )
        .route(routes::v1::results::TESTRUNS, get(get_testruns))
        .route(routes::v1::results::NYM_NODES, get(get_nym_nodes))
}
