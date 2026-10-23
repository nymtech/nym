// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::helpers::PaginationRequestV2;
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::performance::PerformanceEntriesRecordsResponse;
use nym_contract_attestation::SignedDigestSnapshot;
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use tendermint::block::Height;

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
        ))
    ),
)]
async fn latest_performance_snapshot(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedDigestSnapshot>> {
    let snapshot = state
        .performance_snapshots
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
        ))
    ),
)]
async fn performance_snapshot_by_height(
    Path(height): Path<u64>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedDigestSnapshot>> {
    let snapshot = state
        .performance_snapshots
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
        ))
    ),
)]
async fn performance_entries_records(
    Path(height): Path<u64>,
    Query(pagination): Query<PaginationRequestV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<PerformanceEntriesRecordsResponse>> {
    let page = pagination.page.unwrap_or_default();
    let entries = state
        .performance_snapshots
        .paged_entries_at(height, page, pagination.per_page)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find performance entries for height {height}",
            ))
        })?;
    // SAFETY: we just managed to perform a lookup based on this height,
    // so it must be valid
    #[allow(clippy::unwrap_used)]
    Ok(
        pagination.format_response(PerformanceEntriesRecordsResponse {
            height: Height::try_from(height).unwrap(),
            entries,
        }),
    )
}
