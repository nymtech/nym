// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Read-only HTTP endpoints that expose the orchestrator's local database:
//! the set of nym-nodes it is tracking, the test runs it has recorded, and
//! the runs currently in flight.
//!
//! All handlers are thin wrappers that extract path/query parameters and
//! delegate the actual work to [`AppState`]; conversion from storage rows to
//! the public response shapes lives in [`crate::storage::models`]. Every
//! route in this module is protected by the shared `metrics_and_results`
//! bearer token applied one level up in [`crate::http::api::v1::routes`].

use crate::http::api::v1::error::ApiError;
use crate::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use nym_network_monitor_orchestrator_requests::models::{
    NymNodeData, NymNodeWithTestRun, PagedResult, Pagination, TestRunData,
    TestRunsInProgressResponse,
};
use nym_network_monitor_orchestrator_requests::routes;
use nym_validator_client::client::NodeId;

/// Fetches a single completed test run by its database-assigned id.
/// Returns `404` with [`ApiError::TestRunNotFound`] if no such row exists — for
/// example because the run has already been evicted by the stale-result sweeper.
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
async fn get_testrun_by_id(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<TestRunData>, ApiError> {
    state
        .get_testrun_by_id(id)
        .await?
        .map(Json)
        .ok_or(ApiError::TestRunNotFound)
}

/// Fetches a single node along with its most recent completed test run.
///
/// The `latest_test_run` field is `None` if the node has never been tested or
/// if its most recent run has been evicted. Returns `404` with
/// [`ApiError::NymNodeNotFound`] if the orchestrator has never observed a bond
/// for this `node_id`.
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
            (NymNodeWithTestRun = "application/json"),
        ))
    )
)]
async fn get_nym_node_by_id(
    Path(node_id): Path<NodeId>,
    State(state): State<AppState>,
) -> Result<Json<NymNodeWithTestRun>, ApiError> {
    state
        .get_nym_node_by_id(node_id)
        .await?
        .map(Json)
        .ok_or(ApiError::NymNodeNotFound)
}

/// Lists the test runs currently dispatched to agents and awaiting results.
///
/// Ordered oldest-started first, so stale or hung runs surface at the top.
/// The response is capped in the storage layer (at the 200-row defensive limit
/// applied by [`crate::storage::manager::StorageManager::get_all_testruns_in_progress`]);
/// in normal operation the list holds roughly one entry per active agent.
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
async fn get_testruns_in_progress(
    State(state): State<AppState>,
) -> Result<Json<TestRunsInProgressResponse>, ApiError> {
    state.get_testruns_in_progress().await.map(Json)
}

/// Paginated list of all completed test runs, newest first.
///
/// See [`Pagination`] for the page-size/page-number contract and default caps.
/// `total` reflects the row count at the moment the page was read; it is
/// fetched in the same transaction as the page itself to guarantee consistency.
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
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> Result<Json<PagedResult<TestRunData>>, ApiError> {
    state.get_testruns_paginated(pagination).await.map(Json)
}

/// Paginated list of every node the orchestrator has ever observed as bonded,
/// ordered by `node_id` ascending.
///
/// Nodes are only removed from this table if they are explicitly deleted; a
/// node that has unbonded remains visible with its last-known `last_seen_bonded`
/// timestamp.
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
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> Result<Json<PagedResult<NymNodeData>>, ApiError> {
    state.get_nym_nodes_paginated(pagination).await.map(Json)
}

/// Paginated history of test runs for a single node, newest first.
///
/// If `node_id` is unknown or has never been tested the response is a valid
/// empty page (`items: []`, `total: 0`) — there is no 404 here because the
/// orchestrator can't tell from a zero-row result whether the node simply has
/// no runs yet. Backed by the `idx_testrun_node_id_timestamp` index for
/// efficient per-node lookups.
#[utoipa::path(
    operation_id = "v1_results_nym_node_testruns",
    tag = "Network Monitor Results",
    get,
    params(
        ("node_id" = u32, Path, description = "Mixnet-contract node id"),
        Pagination,
    ),
    path = "/nym-node/{node_id}/testruns",
    context_path = "/v1/results",
    security(("metrics_and_results_token" = [])),
    responses(
        (status = 200, content(
            (PagedResult<TestRunData> = "application/json"),
        ))
    )
)]
async fn get_nym_node_testruns(
    Path(node_id): Path<NodeId>,
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> Result<Json<PagedResult<TestRunData>>, ApiError> {
    state
        .get_testruns_for_node_paginated(node_id, pagination)
        .await
        .map(Json)
}

/// Builds the router for the `/v1/results` sub-tree. The caller is expected to
/// nest this under [`routes::v1::RESULTS`] and to attach the shared
/// metrics-and-results bearer-auth layer at the parent level.
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(routes::v1::results::TESTRUN_BY_ID, get(get_testrun_by_id))
        .route(
            routes::v1::results::NYM_NODE_BY_NODE_ID,
            get(get_nym_node_by_id),
        )
        .route(
            routes::v1::results::NYM_NODE_TESTRUNS,
            get(get_nym_node_testruns),
        )
        .route(
            routes::v1::results::TESTRUNS_IN_PROGRESS,
            get(get_testruns_in_progress),
        )
        .route(routes::v1::results::TESTRUNS, get(get_testruns))
        .route(routes::v1::results::NYM_NODES, get(get_nym_nodes))
}
