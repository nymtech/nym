// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::helpers::{NodeIdParam, PaginationRequest};
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_api_requests::models::{
    AnnotationResponse, NodeDatePerformanceResponse, NodePerformanceResponse, NodeRefreshBody,
    NoiseDetails, NymNodeDescription, PerformanceHistoryResponse, RewardedSetResponse,
    UptimeHistoryResponse,
};
use nym_api_requests::pagination::{PaginatedResponse, Pagination};
use nym_contracts_common::NaiveFloat;
use nym_http_api_common::{FormattedResponse, Output, OutputParams};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NymNodeDetails;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::{Date, OffsetDateTime};
use tower_http::compression::CompressionLayer;
use utoipa::{IntoParams, ToSchema};

pub(crate) mod legacy;

pub(crate) fn nym_node_routes() -> Router<AppState> {
    Router::new()
        .route("/refresh-described", post(refresh_described))
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
        .route("/rewarded-set", get(rewarded_set))
        .layer(CompressionLayer::new())
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/rewarded-set",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, content(
            (RewardedSetResponse = "application/json"),
            (RewardedSetResponse = "application/yaml"),
            (RewardedSetResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn rewarded_set(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<RewardedSetResponse>> {
    let output = output.output.unwrap_or_default();

    let rewarded_set = state.nym_contract_cache().rewarded_set_owned().await?;

    Ok(output.to_response(nym_mixnet_contract_common::EpochRewardedSet::from(rewarded_set).into()))
}

#[utoipa::path(
    tag = "Nym Nodes",
    post,
    request_body = NodeRefreshBody,
    path = "/refresh-described",
    context_path = "/v1/nym-nodes",
)]
async fn refresh_described(
    State(state): State<AppState>,
    Json(request_body): Json<NodeRefreshBody>,
) -> AxumResult<Json<()>> {
    let Some(refresh_data) = state
        .nym_contract_cache()
        .get_node_refresh_data(request_body.node_identity)
        .await
    else {
        return Err(AxumErrorResponse::not_found(format!(
            "node with identity {} does not seem to exist",
            request_body.node_identity
        )));
    };

    if !request_body.verify_signature() {
        return Err(AxumErrorResponse::unauthorised("invalid request signature"));
    }

    if request_body.is_stale() {
        return Err(AxumErrorResponse::bad_request("the request is stale"));
    }

    let node_id = refresh_data.node_id();
    if let Some(last) = state.forced_refresh.last_refreshed(node_id).await {
        // max 1 refresh a minute
        let minute_ago = OffsetDateTime::now_utc() - Duration::from_secs(60);
        if last > minute_ago {
            return Err(AxumErrorResponse::too_many(
                "already refreshed node in the last minute",
            ));
        }
    }
    // to make sure you can't ddos the endpoint while a request is in progress
    state.forced_refresh.set_last_refreshed(node_id).await;
    let allow_all_ips = state.forced_refresh.allow_all_ip_addresses;

    if let Some(updated_data) = refresh_data.try_refresh(allow_all_ips).await {
        let Ok(mut describe_cache) = state.described_nodes_cache.write().await else {
            return Err(AxumErrorResponse::service_unavailable());
        };
        describe_cache.get_mut().force_update(updated_data)
    } else {
        return Err(AxumErrorResponse::unprocessable_entity(
            "failed to refresh node description",
        ));
    }

    Ok(Json(()))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/noise",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, content(
            (PaginatedResponse<NoiseDetails> = "application/json"),
            (PaginatedResponse<NoiseDetails> = "application/yaml"),
            (PaginatedResponse<NoiseDetails> = "application/bincode")
        ))
    ),
    params(PaginationRequest)
)]
async fn nodes_noise(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<FormattedResponse<PaginatedResponse<NoiseDetails>>> {
    // TODO: implement it
    let _ = pagination;
    let output = pagination.output.unwrap_or_default();

    let describe_cache = state.describe_nodes_cache_data().await?;

    let nodes = describe_cache
        .all_nodes()
        .filter_map(|n| {
            n.description
                .host_information
                .keys
                .x25519_versioned_noise
                .map(|noise_key| (noise_key, n))
        })
        .map(|(noise_key, node)| NoiseDetails {
            key: noise_key,
            mixnet_port: node.description.mix_port(),
            ip_addresses: node.description.host_information.ip_address.clone(),
        })
        .collect::<Vec<_>>();

    let total = nodes.len();

    Ok(output.to_response(PaginatedResponse {
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
        (status = 200, content(
            (PaginatedResponse<NymNodeDetails> = "application/json"),
            (PaginatedResponse<NymNodeDetails> = "application/yaml"),
            (PaginatedResponse<NymNodeDetails> = "application/bincode")
        ))
    ),
    params(PaginationRequest)
)]
async fn get_bonded_nodes(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> FormattedResponse<PaginatedResponse<NymNodeDetails>> {
    // TODO: implement it
    let _ = pagination;
    let output = pagination.output.unwrap_or_default();

    let details = state.nym_contract_cache().nym_nodes().await;
    let total = details.len();

    output.to_response(PaginatedResponse {
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
        (status = 200, content(
            (PaginatedResponse<NymNodeDescription> = "application/json"),
            (PaginatedResponse<NymNodeDescription> = "application/yaml"),
            (PaginatedResponse<NymNodeDescription> = "application/bincode")
        ))
    ),
    params(PaginationRequest)
)]
async fn get_described_nodes(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<FormattedResponse<PaginatedResponse<NymNodeDescription>>> {
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

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/annotation/{node_id}",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, content(
            (AnnotationResponse = "application/json"),
            (AnnotationResponse = "application/yaml"),
            (AnnotationResponse = "application/bincode")
        ))
    ),
    params(NodeIdParam, OutputParams),
)]
async fn get_node_annotation(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<AnnotationResponse>> {
    let output = output.output.unwrap_or_default();

    let annotations = state
        .node_status_cache
        .node_annotations()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    Ok(output.to_response(AnnotationResponse {
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
        (status = 200, content(
            (NodePerformanceResponse = "application/json"),
            (NodePerformanceResponse = "application/yaml"),
            (NodePerformanceResponse = "application/bincode")
        ))
    ),
    params(NodeIdParam, OutputParams),
)]
async fn get_current_node_performance(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<NodePerformanceResponse>> {
    let output = output.output.unwrap_or_default();

    let annotations = state
        .node_status_cache
        .node_annotations()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    Ok(output.to_response(NodePerformanceResponse {
        node_id,
        performance: annotations
            .get(&node_id)
            .map(|n| n.last_24h_performance.naive_to_f64()),
    }))
}

// todo; probably extract it to requests crate
#[derive(Debug, Serialize, Deserialize, Copy, Clone, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub(crate) struct DateQuery {
    #[schema(value_type = String, example = "1970-01-01")]
    pub(crate) date: Date,

    pub(crate) output: Option<Output>,
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/historical-performance/{node_id}",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, content(
            (NodeDatePerformanceResponse = "application/json"),
            (NodeDatePerformanceResponse = "application/yaml"),
            (NodeDatePerformanceResponse = "application/bincode")
        ))
    ),
    params(DateQuery, NodeIdParam)
)]
async fn get_historical_performance(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    Query(DateQuery { date, output }): Query<DateQuery>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<NodeDatePerformanceResponse>> {
    let output = output.unwrap_or_default();

    let uptime = state
        .storage()
        .get_historical_node_uptime_on(node_id, date)
        .await?;

    Ok(output.to_response(NodeDatePerformanceResponse {
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
        (status = 200, content(
            (PerformanceHistoryResponse = "application/json"),
            (PerformanceHistoryResponse = "application/yaml"),
            (PerformanceHistoryResponse = "application/bincode")
        ))
    ),
    params(PaginationRequest, NodeIdParam)
)]
async fn get_node_performance_history(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<FormattedResponse<PerformanceHistoryResponse>> {
    // TODO: implement it
    let _ = pagination;
    let output = pagination.output.unwrap_or_default();

    let history = state
        .storage()
        .get_node_uptime_history(node_id)
        .await?
        .into_iter()
        .filter_map(|u| u.try_into().ok())
        .collect::<Vec<_>>();
    let total = history.len();

    Ok(output.to_response(PerformanceHistoryResponse {
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
        (status = 200, content(
            (PerformanceHistoryResponse = "application/json"),
            (PerformanceHistoryResponse = "application/yaml"),
            (PerformanceHistoryResponse = "application/bincode")
        ))
    ),
    params(PaginationRequest, NodeIdParam)
)]
async fn get_node_uptime_history(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<FormattedResponse<UptimeHistoryResponse>> {
    // TODO: implement it
    let _ = pagination;
    let output = pagination.output.unwrap_or_default();

    let history = state
        .storage()
        .get_node_uptime_history(node_id)
        .await?
        .into_iter()
        .filter_map(|u| u.try_into().ok())
        .collect::<Vec<_>>();
    let total = history.len();

    Ok(output.to_response(UptimeHistoryResponse {
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
