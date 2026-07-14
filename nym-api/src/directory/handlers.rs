// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use nym_directory_attestation::{
    AttestedDirectoryData, AttestedSubset, SignedDigestSnapshot, SignedSubsetDigest,
};
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
            Router::new().route("/{height}", get(directory_entries)),
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
    path = "/entries/{height}",
    context_path = "/v1/directory",
    responses(
        (status = 200, content(
            (AttestedDirectoryData = "application/json"),
            (AttestedDirectoryData = "application/yaml"),
        ))
    ),
)]
async fn directory_entries(
    Path(height): Path<u64>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<AttestedDirectoryData>> {
    let entries = state.directory.entries_at(height).await.ok_or_else(|| {
        AxumErrorResponse::not_found(format!(
            "could not find directory entries for height {height}",
        ))
    })?;
    Ok(output.to_response(entries))
}
