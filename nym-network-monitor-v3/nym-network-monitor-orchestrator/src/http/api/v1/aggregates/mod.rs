// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Read-only HTTP endpoints serving the materialised per-epoch aggregates. The runs behind them are
//! served per node and kind by the results endpoints. Every route here is protected by the shared
//! `metrics_and_results` bearer token applied one level up in [`crate::http::api::v1::routes`], the
//! same as the results endpoints.

use crate::http::api::v1::error::ApiError;
use crate::http::state::AppState;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use nym_network_monitor_orchestrator_requests::models::NodeEpochAggregates;
use nym_network_monitor_orchestrator_requests::routes;
use nym_validator_client::client::NodeId;

/// Every node's aggregates for one mixnet epoch, one record per node.
///
/// Not paginated: the population is around a thousand nodes and each record is small. A node that
/// returned nothing in the epoch's window is simply absent, since an aggregate is only ever written
/// where something was measured.
#[utoipa::path(
    operation_id = "v1_aggregates_epoch",
    tag = "Network Monitor Aggregates",
    get,
    params(("mixnet_epoch" = u32, Path, description = "Absolute mixnet epoch id")),
    path = "/epoch/{mixnet_epoch}",
    context_path = "/v1/aggregates",
    security(("metrics_and_results_token" = [])),
    responses(
        (status = 200, content(
            (Vec<NodeEpochAggregates> = "application/json"),
        )),
        (status = 500, description = "failed to read aggregates from storage"),
    )
)]
async fn get_epoch_aggregates(
    Path(mixnet_epoch): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<Vec<NodeEpochAggregates>>, ApiError> {
    state
        .get_epoch_aggregates(mixnet_epoch as i64)
        .await
        .map(Json)
}

/// One node's aggregates for one mixnet epoch, a liveness and a stress entry.
///
/// An entry with no value is absent from the record rather than reported as zero. A node with no
/// aggregate at all returns a record with both entries absent rather than a 404: the
/// aggregate table holds no row for an unmeasured node, so it cannot tell "not measured" from "no
/// such node", and the emptiness is the honest answer.
#[utoipa::path(
    operation_id = "v1_aggregates_nym_node_epoch",
    tag = "Network Monitor Aggregates",
    get,
    params(
        ("node_id" = u32, Path, description = "Mixnet-contract node id"),
        ("mixnet_epoch" = u32, Path, description = "Absolute mixnet epoch id"),
    ),
    path = "/nym-node/{node_id}/epoch/{mixnet_epoch}",
    context_path = "/v1/aggregates",
    security(("metrics_and_results_token" = [])),
    responses(
        (status = 200, content(
            (NodeEpochAggregates = "application/json"),
        )),
        (status = 500, description = "failed to read aggregates from storage"),
    )
)]
async fn get_node_epoch_aggregates(
    Path((node_id, mixnet_epoch)): Path<(NodeId, u32)>,
    State(state): State<AppState>,
) -> Result<Json<NodeEpochAggregates>, ApiError> {
    state
        .get_node_epoch_aggregates(mixnet_epoch as i64, node_id)
        .await
        .map(Json)
}

/// Builds the router for the `/v1/aggregates` sub-tree. The caller nests this under
/// [`routes::v1::AGGREGATES`] and attaches the shared metrics-and-results bearer-auth layer at the
/// parent level.
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(routes::v1::aggregates::EPOCH, get(get_epoch_aggregates))
        .route(
            routes::v1::aggregates::NYM_NODE_EPOCH,
            get(get_node_epoch_aggregates),
        )
}
