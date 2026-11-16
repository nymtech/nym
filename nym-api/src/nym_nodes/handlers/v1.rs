// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::nym_nodes::node_view::NodeSubset;
use crate::support::http::helpers::{NodeIdParam, PaginationRequest};
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use nym_api_requests::models::described::v1::NymNodeDescriptionV1;
use nym_api_requests::models::described::NoiseDetails;
use nym_api_requests::models::{
    AnnotationResponseV1, NodeDatePerformanceResponse, NodePerformanceResponse,
    PerformanceHistoryResponse, RewardedSetResponse, StakeSaturationResponse,
    UptimeHistoryResponse,
};
use nym_api_requests::pagination::{PaginatedResponse, Pagination};
use nym_contracts_common::NaiveFloat;
use nym_http_api_common::{FormattedResponse, Output, OutputParams};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NymNodeDetails;
use serde::{Deserialize, Serialize};
use time::Date;
use tower_http::compression::CompressionLayer;
use utoipa::{IntoParams, ToSchema};

#[allow(deprecated)]
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/refresh-described", post(refresh_described))
        .route("/noise", get(nodes_noise))
        .route("/bonded", get(get_bonded_nodes))
        .route("/described", get(get_described_nodes))
        .route("/annotation/{node_id}", get(get_node_annotation))
        .route("/performance/{node_id}", get(get_current_node_performance))
        .route(
            "/stake-saturation/{node_id}",
            get(get_node_stake_saturation),
        )
        .route(
            "/historical-performance/{node_id}",
            get(get_historical_performance),
        )
        .route(
            "/performance-history/{node_id}",
            get(get_node_performance_history),
        )
        // to make it compatible with all the explorers that were used to using 0-100 values
        .route("/uptime-history/{node_id}", get(get_node_uptime_history))
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

    let rewarded_set = state.latest_snapshot().await?.mixnet().rewarded_set.clone();

    Ok(output.to_response(nym_mixnet_contract_common::EpochRewardedSet::from(rewarded_set).into()))
}

/// Removed: nym-api reads node data from the directory, so there is nothing to refresh.
#[utoipa::path(
    tag = "Nym Nodes",
    post,
    path = "/refresh-described",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 410, description = "the endpoint was removed", body = String, content_type = "text/plain")
    ),
)]
#[deprecated]
async fn refresh_described() -> (StatusCode, &'static str) {
    (
        StatusCode::GONE,
        "this endpoint was removed: nym-api reads node data from the directory contract and no \
        longer queries nym-nodes. Please upgrade your nym-node.",
    )
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

    let snapshot = state.latest_snapshot().await?;

    let nodes = snapshot
        .node_view
        .nodes(NodeSubset::All)
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
) -> AxumResult<FormattedResponse<PaginatedResponse<NymNodeDetails>>> {
    // TODO: implement it
    let _ = pagination;
    let output = pagination.output.unwrap_or_default();

    let details = state.latest_snapshot().await?.mixnet().nym_nodes.clone();
    let total = details.len();

    Ok(output.to_response(PaginatedResponse {
        pagination: Pagination {
            total,
            page: 0,
            size: total,
        },
        data: details,
    }))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/described",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, content(
            (PaginatedResponse<NymNodeDescriptionV1> = "application/json"),
            (PaginatedResponse<NymNodeDescriptionV1> = "application/yaml"),
            (PaginatedResponse<NymNodeDescriptionV1> = "application/bincode")
        ))
    ),
    params(PaginationRequest)
)]
#[deprecated(note = "use '/v2/nym-nodes/described' instead")]
async fn get_described_nodes(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationRequest>,
) -> AxumResult<FormattedResponse<PaginatedResponse<NymNodeDescriptionV1>>> {
    // TODO: implement it
    let _ = pagination;
    let output = pagination.output.unwrap_or_default();

    let snapshot = state.latest_snapshot().await?;

    // convert description to V1
    let descriptions = snapshot
        .node_view
        .nodes(NodeSubset::All)
        .map(|d| d.clone().into())
        .collect::<Vec<_>>();

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
            (AnnotationResponseV1 = "application/json"),
            (AnnotationResponseV1 = "application/yaml"),
            (AnnotationResponseV1 = "application/bincode")
        ))
    ),
    params(NodeIdParam, OutputParams),
)]
async fn get_node_annotation(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<AnnotationResponseV1>> {
    let output = output.output.unwrap_or_default();

    let snapshot = state.latest_snapshot().await?;

    Ok(output.to_response(AnnotationResponseV1 {
        node_id,
        annotation: snapshot.node_annotation(node_id).map(Into::into),
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

    let snapshot = state.latest_snapshot().await?;

    Ok(output.to_response(NodePerformanceResponse {
        node_id,
        performance: snapshot.published_score(node_id),
    }))
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/stake-saturation/{node_id}",
    context_path = "/v1/nym-nodes",
    responses(
        (status = 200, content(
            (StakeSaturationResponse = "application/json"),
            (StakeSaturationResponse = "application/yaml"),
            (StakeSaturationResponse = "application/bincode")
        ))
    ),
    params(NodeIdParam, OutputParams),
)]
async fn get_node_stake_saturation(
    Path(NodeIdParam { node_id }): Path<NodeIdParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<StakeSaturationResponse>> {
    let output = output.get_output();

    let snapshot = state.latest_snapshot().await?;
    let mixnet = snapshot.mixnet();
    let Some(node) = mixnet
        .nym_nodes
        .iter()
        .find(|node| node.node_id() == node_id)
    else {
        return Err(AxumErrorResponse::not_found("nym node bond not found"));
    };

    let rewarding_params = &mixnet.current_reward_params;

    Ok(output.to_response(StakeSaturationResponse {
        saturation: node.rewarding_details.bond_saturation(rewarding_params),
        uncapped_saturation: node
            .rewarding_details
            .uncapped_bond_saturation(rewarding_params),
        as_at: snapshot.block_time().unix_timestamp(),
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

/// Deprecated: the v1 network monitor that recorded this data is gone, so it no longer changes.
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
#[deprecated]
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

/// Deprecated: the v1 network monitor that recorded this data is gone, so it no longer changes.
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
#[deprecated]
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

/// Deprecated: the v1 network monitor that recorded this data is gone, so it no longer changes.
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
#[deprecated]
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
