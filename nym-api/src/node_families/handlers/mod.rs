// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::support::http::helpers::PaginationRequestV2;
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::node_families::{
    NodeFamily, NodeFamilyForNodeResponse, NodeFamilyResponse,
};
use nym_api_requests::pagination::{PaginatedResponse, Pagination};
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use nym_mixnet_contract_common::NodeId;
use nym_node_families_contract_common::NodeFamilyId;
use std::cmp::min;

const DEFAULT_FAMILIES_PAGE_SIZE: u32 = 50;
const MAX_FAMILIES_PAGE_SIZE: u32 = 200;

// /v1/node-families
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_families))
        .route("/:family_id", get(get_family_by_id))
        .route("/by-node/:node_id", get(get_family_for_node))
}

#[utoipa::path(
    tag = "Node Families",
    get,
    path = "",
    context_path = "/v1/node-families",
    responses(
        (status = 200, content(
            (PaginatedResponse<NodeFamily> = "application/json"),
            (PaginatedResponse<NodeFamily> = "application/yaml"),
        ))
    ),
    params(PaginationRequestV2)
)]
async fn get_families(
    Query(pagination): Query<PaginationRequestV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<PaginatedResponse<NodeFamily>>> {
    let page = pagination.page.unwrap_or_default();
    let per_page = min(
        pagination.per_page.unwrap_or(DEFAULT_FAMILIES_PAGE_SIZE),
        MAX_FAMILIES_PAGE_SIZE,
    );
    let output = pagination.output.unwrap_or_default();

    let cache = state.node_families_cache.get().await?;
    let total = cache.families.len();
    let offset = (page as usize).saturating_mul(per_page as usize);

    // BTreeMap ascending-id iteration is stable across refreshes, so paging
    // by offset is well-defined: page N is the same window of ids on every
    // call (until the underlying set changes).
    let data: Vec<NodeFamily> = cache
        .families
        .values()
        .skip(offset)
        .take(per_page as usize)
        .map(Into::into)
        .collect();

    Ok(output.to_response(PaginatedResponse {
        pagination: Pagination {
            total,
            page,
            size: data.len(),
        },
        data,
    }))
}

#[utoipa::path(
    tag = "Node Families",
    get,
    path = "/{family_id}",
    context_path = "/v1/node-families",
    responses(
        (status = 200, content(
            (NodeFamilyResponse = "application/json"),
            (NodeFamilyResponse = "application/yaml"),
        ))
    ),
    params(
        ("family_id" = u32, Path, description = "Identifier of the family"),
        OutputParamsV2,
    )
)]
async fn get_family_by_id(
    Path(family_id): Path<NodeFamilyId>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<NodeFamilyResponse>> {
    let output = output.get_output();

    let cache = state.node_families_cache.get().await?;
    let family = cache.families.get(&family_id).map(NodeFamily::from);

    Ok(output.to_response(NodeFamilyResponse { family }))
}

#[utoipa::path(
    tag = "Node Families",
    get,
    path = "/by-node/{node_id}",
    context_path = "/v1/node-families",
    responses(
        (status = 200, content(
            (NodeFamilyForNodeResponse = "application/json"),
            (NodeFamilyForNodeResponse = "application/yaml"),
        ))
    ),
    params(
        ("node_id" = u32, Path, description = "Identifier of the member node"),
        OutputParamsV2,
    )
)]
async fn get_family_for_node(
    Path(node_id): Path<NodeId>,
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<NodeFamilyForNodeResponse>> {
    let output = output.get_output();

    let cache = state.node_families_cache.get().await?;
    let family = cache
        .family_by_member
        .get(&node_id)
        .and_then(|family_id| cache.families.get(family_id))
        .map(NodeFamily::from);

    Ok(output.to_response(NodeFamilyForNodeResponse { node_id, family }))
}
