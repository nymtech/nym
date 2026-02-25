// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::support::http::helpers::PaginationRequest;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::NymNodeDescriptionV2;
use nym_api_requests::pagination::{PaginatedResponse, Pagination};
use nym_http_api_common::FormattedResponse;
use tower_http::compression::CompressionLayer;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/described", get(get_described_nodes))
        .layer(CompressionLayer::new())
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/described",
    context_path = "/v2/nym-nodes",
    responses(
        (status = 200, content(
            (PaginatedResponse<NymNodeDescriptionV2> = "application/json"),
            (PaginatedResponse<NymNodeDescriptionV2> = "application/yaml"),
            (PaginatedResponse<NymNodeDescriptionV2> = "application/bincode")
        ))
    ),
    params(PaginationRequest)
)]
async fn get_described_nodes(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<FormattedResponse<PaginatedResponse<NymNodeDescriptionV2>>> {
    // TODO: implement it
    let _ = pagination;
    let output = pagination.output.unwrap_or_default();

    let cache = state.described_nodes_cache.get().await?;
    let descriptions = cache.all_nodes().cloned().collect::<Vec<_>>();

    Ok(output.to_response(PaginatedResponse {
        pagination: Pagination {
            total: descriptions.len(),
            page: 0,
            size: descriptions.len(),
        },
        data: descriptions,
    }))
}
