// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::helpers::PaginationRequestV2;
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::directory::{
    DirectoryEntriesIdentitiesResponse, DirectoryEntriesRecordsResponse,
};
use nym_directory_attestation::{AttestedSubset, SignedDigestSnapshot, SignedSubsetDigest};
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use std::cmp::min;
use tendermint::block::Height;

const DEFAULT_ENTRIES_PAGE_SIZE: u32 = 200;
const MAX_ENTRIES_PAGE_SIZE: u32 = 500;

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
                    "/{height}/node_identities",
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
        ))
    ),
)]
async fn latest_snapshot(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedDigestSnapshot>> {
    let snapshot = state.directory.latest_snapshot().await.ok_or_else(|| {
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
        ))
    ),
)]
async fn snapshot_by_height(
    Path(height): Path<u64>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedDigestSnapshot>> {
    let snapshot = state.directory.snapshot_at(height).await.ok_or_else(|| {
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
        ))
    ),
)]
async fn directory_subset_digest(
    Path((subset_id, height)): Path<(String, u64)>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SignedSubsetDigest>> {
    let digest = state
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
        ))
    ),
)]
async fn directory_subset(
    Path((subset_id, height)): Path<(String, u64)>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<AttestedSubset>> {
    let subset = state
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
        ))
    ),
)]
async fn directory_entries_records(
    Path(height): Path<u64>,
    Query(pagination): Query<PaginationRequestV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<DirectoryEntriesRecordsResponse>> {
    let page = pagination.page.unwrap_or_default();
    let per_page = min(
        pagination.per_page.unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE),
        MAX_ENTRIES_PAGE_SIZE,
    );
    let entries = state
        .directory
        .paged_entries_at(height, page, per_page)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find directory entries for height {height}",
            ))
        })?;
    // SAFETY: we just managed to perform a lookup based on this height,
    // so it must be valid
    #[allow(clippy::unwrap_used)]
    Ok(pagination.to_response(DirectoryEntriesRecordsResponse {
        height: Height::try_from(height).unwrap(),
        entries,
    }))
}

#[utoipa::path(
    tag = "Nym Directory",
    get,
    path = "/entries/{height}/records",
    context_path = "/v1/directory",
    responses(
        (status = 200, content(
            (DirectoryEntriesIdentitiesResponse = "application/json"),
            (DirectoryEntriesIdentitiesResponse = "application/yaml"),
        ))
    ),
)]
async fn directory_entries_identities(
    Path(height): Path<u64>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<DirectoryEntriesIdentitiesResponse>> {
    let node_identities = state
        .directory
        .node_identities_at(height)
        .await
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!(
                "could not find directory identities for height {height}",
            ))
        })?;
    // SAFETY: we just managed to perform a lookup based on this height,
    // so it must be valid
    #[allow(clippy::unwrap_used)]
    Ok(output.to_response(DirectoryEntriesIdentitiesResponse {
        height: Height::try_from(height).unwrap(),
        node_identities,
    }))
}
