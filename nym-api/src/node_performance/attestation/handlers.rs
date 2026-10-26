// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::helpers::{snapshot_height, PaginationRequestV2};
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::performance::PerformanceEntriesRecordsResponse;
use nym_contract_attestation::SignedDigestSnapshot;
use nym_http_api_common::{FormattedResponse, OutputParamsV2};

// /v1/performance
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .nest(
            "/snapshot",
            Router::new()
                .route(
                    "/",
                    get(|| async { Redirect::to("/v1/performance/snapshot/latest") }),
                )
                .route("/latest", get(latest_performance_snapshot))
                .route("/{height}", get(performance_snapshot_by_height)),
        )
        // whole-set transfer: every committed entry at a height, so a client can recompute the
        // accumulator offline against a quorum'd snapshot. The node identities at that height
        // are the directory's (`/v1/directory/entries/{height}/node-identities`).
        .nest(
            "/entries",
            Router::new().route("/{height}/records", get(performance_entries_records)),
        )
}

#[utoipa::path(
    tag = "Nym Performance",
    get,
    path = "/snapshot/latest",
    context_path = "/v1/performance",
    responses(
        (status = 200, content(
            (SignedDigestSnapshot = "application/json"),
            (SignedDigestSnapshot = "application/yaml"),
        )),
        (status = 503, description = "no performance snapshot has settled yet"),
    ),
    params(OutputParamsV2),
)]
async fn latest_performance_snapshot(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedDigestSnapshot>> {
    let snapshot = state
        .contract_snapshots
        .performance
        .latest_snapshot()
        .await
        .ok_or_else(|| {
            AxumErrorResponse::service_unavailable("no performance snapshots available")
        })?;
    Ok(output.to_response(snapshot))
}

#[utoipa::path(
    tag = "Nym Performance",
    get,
    path = "/snapshot/{height}",
    context_path = "/v1/performance",
    responses(
        (status = 200, content(
            (SignedDigestSnapshot = "application/json"),
            (SignedDigestSnapshot = "application/yaml"),
        )),
        (status = 404, description = "the height is not a retained snapshot height"),
    ),
    params(
        ("height" = u64, Path, description = "Height of the snapshot"),
        OutputParamsV2
    ),
)]
async fn performance_snapshot_by_height(
    Path(height): Path<u64>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedDigestSnapshot>> {
    let height = snapshot_height(height)?;
    let snapshot = state
        .contract_snapshots
        .performance
        .snapshot_at(height)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find performance snapshot for height {height}",
            ))
        })?;
    Ok(output.to_response(snapshot))
}

#[utoipa::path(
    tag = "Nym Performance",
    get,
    path = "/entries/{height}/records",
    context_path = "/v1/performance",
    responses(
        (status = 200, content(
            (PerformanceEntriesRecordsResponse = "application/json"),
            (PerformanceEntriesRecordsResponse = "application/yaml"),
        )),
        (status = 404, description = "the height is not a retained snapshot height"),
    ),
    params(
        ("height" = u64, Path, description = "Height of the snapshot"),
        PaginationRequestV2
    ),
)]
async fn performance_entries_records(
    Path(height): Path<u64>,
    Query(pagination): Query<PaginationRequestV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<PerformanceEntriesRecordsResponse>> {
    let height = snapshot_height(height)?;
    let page = pagination.page.unwrap_or_default();
    let entries = state
        .contract_snapshots
        .performance
        .paged_entries_at(height, page, pagination.per_page)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find performance entries for height {height}",
            ))
        })?;
    Ok(pagination.format_response(PerformanceEntriesRecordsResponse { height, entries }))
}
