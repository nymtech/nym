// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::helpers::{NodeIdParam, PaginationRequest};
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use nym_api_requests::models::{
    AnnotationResponse, NodeDatePerformanceResponse, NodePerformanceResponse, NoiseDetails,
    NymNodeData, PerformanceHistoryResponse, UptimeHistoryResponse,
};
use nym_api_requests::pagination::{PaginatedResponse, Pagination};
use nym_contracts_common::NaiveFloat;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NymNodeDetails;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::Date;
use utoipa::{IntoParams, ToSchema};

pub(crate) mod legacy;
pub(crate) mod unstable;

pub(crate) fn nym_node_routes() -> Router<AppState> {
    Router::new()
        .route("/noise", get(nodes_noise))
        .route("/bonded", get(get_bonded_nodes))
        .route("/described", get(get_described_nodes))
        .route("/annotation/:node_id", get(get_node_annotation))
        .route("/performance/:node_id", get(get_current_node_performance))
        .route(
            "/historical-performance/:node_id",
            get(get_historical_performance),
        )
        .route(
            "/performance-history/:node_id",
            get(get_node_performance_history),
        )
        // to make it compatible with all the explorers that were used to using 0-100 values
        .route("/uptime-history/:node_id", get(get_node_uptime_history))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/noise",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, body = PaginatedResponse<NoiseDetails>)
    ),
    params(PaginationRequest)
)]
async fn nodes_noise(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<Json<PaginatedResponse<NoiseDetails>>> {
    // TODO: implement it
    let _ = pagination;

    let describe_cache = state.describe_nodes_cache_data().await?;

    let nodes = describe_cache
        .all_nodes()
        .filter_map(|n| {
            n.description
                .host_information
                .keys
                .x25519_noise
                .map(|noise_key| (noise_key, n))
        })
        .map(|(noise_key, node)| NoiseDetails {
            x25119_pubkey: noise_key,
            mixnet_port: node.description.mix_port(),
            ip_addresses: node.description.host_information.ip_address.clone(),
        })
        .collect::<Vec<_>>();

    let total = nodes.len();

    Ok(Json(PaginatedResponse {
        pagination: Pagination {
            total,
            page: 0,
            size: total,
        },
        data: nodes,
    }))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/bonded",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, body = PaginatedResponse<NymNodeDetails>)
    ),
    params(PaginationRequest)
)]
async fn get_bonded_nodes(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> Json<PaginatedResponse<NymNodeDetails>> {
    // TODO: implement it
    let _ = pagination;

    let details = state.nym_contract_cache().nym_nodes().await;
    let total = details.len();

    Json(PaginatedResponse {
        pagination: Pagination {
            total,
            page: 0,
            size: total,
        },
        data: details,
    })
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/described",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, body = PaginatedResponse<NymNodeData>)
    ),
    params(PaginationRequest)
)]
async fn get_described_nodes(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<Json<PaginatedResponse<NymNodeData>>> {
    // TODO: implement it
    let _ = pagination;

    let cache = state.described_nodes_cache.get().await?;
    let descriptions = cache.all_nodes();

    let data = descriptions
        .map(|n| &n.description)
        .cloned()
        .collect::<Vec<_>>();

    Ok(Json(PaginatedResponse {
        pagination: Pagination {
            total: data.len(),
            page: 0,
            size: data.len(),
        },
        data,
    }))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/annotation/{node_id}",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, body = AnnotationResponse)
    ),
    params(NodeIdParam),
)]
async fn get_node_annotation(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    State(state): State<AppState>,
) -> AxumResult<Json<AnnotationResponse>> {
    let annotations = state
        .node_status_cache
        .node_annotations()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    Ok(Json(AnnotationResponse {
        node_id,
        annotation: annotations.get(&node_id).cloned(),
    }))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/performance/{node_id}",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, body = NodePerformanceResponse)
    ),
    params(NodeIdParam),
)]
async fn get_current_node_performance(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    State(state): State<AppState>,
) -> AxumResult<Json<NodePerformanceResponse>> {
    let annotations = state
        .node_status_cache
        .node_annotations()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    Ok(Json(NodePerformanceResponse {
        node_id,
        performance: annotations
            .get(&node_id)
            .map(|n| n.last_24h_performance.naive_to_f64()),
    }))
}

// todo; probably extract it to requests crate
#[derive(Debug, Serialize, Deserialize, Copy, Clone, IntoParams, ToSchema, JsonSchema)]
#[into_params(parameter_in = Query)]
pub(crate) struct DateQuery {
    #[schema(value_type = String, example = "1970-01-01")]
    #[schemars(with = "String")]
    pub(crate) date: Date,
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/historical-performance/{node_id}",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, body = NodeDatePerformanceResponse)
    ),
    params(DateQuery, NodeIdParam)
)]
async fn get_historical_performance(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    Query(DateQuery { date }): Query<DateQuery>,
    State(state): State<AppState>,
) -> AxumResult<Json<NodeDatePerformanceResponse>> {
    let uptime = state
        .storage()
        .get_historical_node_uptime_on(node_id, date)
        .await?;

    Ok(Json(NodeDatePerformanceResponse {
        node_id,
        date,
        performance: uptime.and_then(|u| {
            Performance::from_percentage_value(u.uptime as u64)
                .map(|p| p.naive_to_f64())
                .ok()
        }),
    }))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/performance-history/{node_id}",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, body = PerformanceHistoryResponse)
    ),
    params(PaginationRequest, NodeIdParam)
)]
async fn get_node_performance_history(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<Json<PerformanceHistoryResponse>> {
    // TODO: implement it
    let _ = pagination;

    let history = state
        .storage()
        .get_node_uptime_history(node_id)
        .await?
        .into_iter()
        .filter_map(|u| u.try_into().ok())
        .collect::<Vec<_>>();
    let total = history.len();

    Ok(Json(PerformanceHistoryResponse {
        node_id,
        history: PaginatedResponse {
            pagination: Pagination {
                total,
                page: 0,
                size: total,
            },
            data: history,
        },
    }))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/uptime-history/{node_id}",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, body = PerformanceHistoryResponse)
    ),
    params(PaginationRequest, NodeIdParam)
)]
async fn get_node_uptime_history(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<Json<UptimeHistoryResponse>> {
    // TODO: implement it
    let _ = pagination;

    let history = state
        .storage()
        .get_node_uptime_history(node_id)
        .await?
        .into_iter()
        .filter_map(|u| u.try_into().ok())
        .collect::<Vec<_>>();
    let total = history.len();

    Ok(Json(UptimeHistoryResponse {
        node_id,
        history: PaginatedResponse {
            pagination: Pagination {
                total,
                page: 0,
                size: total,
            },
            data: history,
        },
    }))
}
