// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::helpers::{snapshot_height, PaginationRequestV2};
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::directory::{
    DirectoryEntriesIdentitiesResponse, DirectoryEntriesRecordsResponse,
};
use nym_contract_attestation::{AttestedSubset, SignedDigestSnapshot, SignedSubsetDigest};
use nym_http_api_common::{FormattedResponse, OutputParamsV2};

// /v1/directory
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .nest(
            "/snapshot",
            Router::new()
                .route(
                    "/",
                    get(|| async { Redirect::to("/v1/directory/snapshot/latest") }),
                )
                .route("/latest", get(latest_snapshot))
                .route("/{height}", get(snapshot_by_height)),
        )
        .nest(
            "/subset",
            Router::new()
                .route("/digest/{subset_id}/{height}", get(directory_subset_digest))
                .route("/data/{subset_id}/{height}", get(directory_subset)),
        )
        // whole-directory transfer: the full entry set + node identities at a height, so a
        // client can recompute the accumulator offline against a quorum'd snapshot. By height
        // only (the client drives it with the snapshot's height); unsigned - the snapshot is
        // the commitment.
        .nest(
            "/entries",
            Router::new()
                .route("/{height}/records", get(directory_entries_records))
                .route(
                    "/{height}/node-identities",
                    get(directory_entries_identities),
                ),
        )
        // routes exposing human-readable json data with backing signature (no canonical encoding due to json)
        .nest("/unattested", Router::new())
}

#[utoipa::path(
    tag = "Nym Directory",
    get,
    path = "/snapshot/latest",
    context_path = "/v1/directory",
    responses(
        (status = 200, content(
            (SignedDigestSnapshot = "application/json"),
            (SignedDigestSnapshot = "application/yaml"),
        )),
        (status = 503, description = "no directory snapshot has settled yet"),
    ),
    params(OutputParamsV2),
)]
async fn latest_snapshot(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedDigestSnapshot>> {
    let snapshot = state
        .contract_snapshots
        .directory
        .latest_snapshot()
        .await
        .ok_or_else(|| {
            AxumErrorResponse::service_unavailable("no directory snapshots available")
        })?;
    Ok(output.to_response(snapshot))
}

#[utoipa::path(
    tag = "Nym Directory",
    get,
    path = "/snapshot/{height}",
    context_path = "/v1/directory",
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
async fn snapshot_by_height(
    Path(height): Path<u64>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedDigestSnapshot>> {
    let height = snapshot_height(height)?;
    let snapshot = state
        .contract_snapshots
        .directory
        .snapshot_at(height)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find digest snapshot for height {height}",
            ))
        })?;
    Ok(output.to_response(snapshot))
}

#[utoipa::path(
    tag = "Nym Directory",
    get,
    path = "/subset/digest/{subset_id}/{height}",
    context_path = "/v1/directory",
    responses(
        (status = 200, content(
            (SignedSubsetDigest = "application/json"),
            (SignedSubsetDigest = "application/yaml"),
        )),
        (status = 404, description = "no such subset at a retained snapshot height"),
    ),
    params(
        ("subset_id" = String, Path, description = "Identifier of the subset"),
        ("height" = u64, Path, description = "Height of the snapshot"),
        OutputParamsV2
    ),
)]
async fn directory_subset_digest(
    Path((subset_id, height)): Path<(String, u64)>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedSubsetDigest>> {
    let height = snapshot_height(height)?;
    let digest = state
        .contract_snapshots
        .directory
        .directory_subset_digest(&subset_id, height)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find digest snapshot for height {height}",
            ))
        })?;
    Ok(output.to_response(digest))
}

#[utoipa::path(
    tag = "Nym Directory",
    get,
    path = "/subset/data/{subset_id}/{height}",
    context_path = "/v1/directory",
    responses(
        (status = 200, content(
            (AttestedSubset = "application/json"),
            (AttestedSubset = "application/yaml"),
        )),
        (status = 404, description = "no such subset at a retained snapshot height"),
    ),
    params(
        ("subset_id" = String, Path, description = "Identifier of the subset"),
        ("height" = u64, Path, description = "Height of the snapshot"),
        OutputParamsV2
    ),
)]
async fn directory_subset(
    Path((subset_id, height)): Path<(String, u64)>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<AttestedSubset>> {
    let height = snapshot_height(height)?;
    let subset = state
        .contract_snapshots
        .directory
        .directory_subset(&subset_id, height)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find subset snapshot for height {height}",
            ))
        })?;
    Ok(output.to_response(subset))
}

#[utoipa::path(
    tag = "Nym Directory",
    get,
    path = "/entries/{height}/records",
    context_path = "/v1/directory",
    responses(
        (status = 200, content(
            (DirectoryEntriesRecordsResponse = "application/json"),
            (DirectoryEntriesRecordsResponse = "application/yaml"),
        )),
        (status = 404, description = "the height is not a retained snapshot height"),
    ),
    params(
        ("height" = u64, Path, description = "Height of the snapshot"),
        PaginationRequestV2
    ),
)]
async fn directory_entries_records(
    Path(height): Path<u64>,
    Query(pagination): Query<PaginationRequestV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<DirectoryEntriesRecordsResponse>> {
    let height = snapshot_height(height)?;
    let page = pagination.page.unwrap_or_default();
    let entries = state
        .contract_snapshots
        .directory
        .paged_entries_at(height, page, pagination.per_page)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find directory entries for height {height}",
            ))
        })?;
    Ok(pagination.format_response(DirectoryEntriesRecordsResponse { height, entries }))
}

#[utoipa::path(
    tag = "Nym Directory",
    get,
    path = "/entries/{height}/node-identities",
    context_path = "/v1/directory",
    responses(
        (status = 200, content(
            (DirectoryEntriesIdentitiesResponse = "application/json"),
            (DirectoryEntriesIdentitiesResponse = "application/yaml"),
        )),
        (status = 404, description = "the height is not a retained snapshot height"),
    ),
    params(
        ("height" = u64, Path, description = "Height of the snapshot"),
        OutputParamsV2
    ),
)]
async fn directory_entries_identities(
    Path(height): Path<u64>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<DirectoryEntriesIdentitiesResponse>> {
    let height = snapshot_height(height)?;
    let node_identities = state
        .contract_snapshots
        .directory
        .node_identities_at(height)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find directory identities for height {height}",
            ))
        })?;
    Ok(output.to_response(DirectoryEntriesIdentitiesResponse {
        height,
        node_identities,
    }))
}
