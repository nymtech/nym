// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::helpers::PaginationRequestV2;
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::node_families::NodeFamily;
use nym_api_requests::pagination::{PaginatedResponse, Pagination};
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use nym_mixnet_contract_common::NodeId;
use nym_node_families_contract_common::NodeFamilyId;

// /v1/node-families
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_families))
        .route("/{family_id}", get(get_family_by_id))
        .route("/by-node/{node_id}", get(get_family_for_node))
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
            (PaginatedResponse<NodeFamily> = "application/bincode")
        ))
    ),
    params(PaginationRequestV2)
)]
async fn get_families(
    Query(pagination): Query<PaginationRequestV2>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<PaginatedResponse<NodeFamily>>> {
    // TODO: paginate
    let _ = pagination;
    let output = pagination.output.unwrap_or_default();

    let cache = state.node_families_cache.get().await?;
    let families: Vec<NodeFamily> = cache.families.iter().map(Into::into).collect();

    Ok(output.to_response(PaginatedResponse {
        pagination: Pagination {
            total: families.len(),
            page: 0,
            size: families.len(),
        },
        data: families,
    }))
}

#[utoipa::path(
    tag = "Node Families",
    get,
    path = "/{family_id}",
    context_path = "/v1/node-families",
    responses(
        (status = 200, content(
            (NodeFamily = "application/json"),
            (NodeFamily = "application/yaml"),
            (NodeFamily = "application/bincode")
        )),
        (status = 404, description = "no family with the requested id exists in the cache")
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
) -> AxumResult<FormattedResponse<NodeFamily>> {
    let output = output.get_output();

    let cache = state.node_families_cache.get().await?;
    let family = cache
        .families
        .iter()
        .find(|f| f.id == family_id)
        .map(NodeFamily::from)
        .ok_or_else(|| AxumErrorResponse::not_found(format!("family {family_id} not found")))?;

    Ok(output.to_response(family))
}

#[utoipa::path(
    tag = "Node Families",
    get,
    path = "/by-node/{node_id}",
    context_path = "/v1/node-families",
    responses(
        (status = 200, content(
            (NodeFamily = "application/json"),
            (NodeFamily = "application/yaml"),
            (NodeFamily = "application/bincode")
        )),
        (status = 404, description = "node is not a member of any cached family")
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
) -> AxumResult<FormattedResponse<NodeFamily>> {
    let output = output.get_output();

    let cache = state.node_families_cache.get().await?;
    let family = cache
        .families
        .iter()
        .find(|f| f.members.iter().any(|m| m.node_id == node_id))
        .map(NodeFamily::from)
        .ok_or_else(|| {
            AxumErrorResponse::not_found(format!("node {node_id} is not a member of any family"))
        })?;

    Ok(output.to_response(family))
}
