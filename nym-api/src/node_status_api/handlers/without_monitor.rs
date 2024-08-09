// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::helpers::{
    _get_active_set_detailed, _get_mixnode_inclusion_probabilities,
    _get_mixnode_inclusion_probability, _get_mixnode_stake_saturation, _get_mixnode_status,
    _get_mixnodes_detailed, _get_rewarded_set_detailed,
};
use crate::node_status_api::models::AxumResult;
use crate::v2::AxumAppState;
use axum::extract::{Path, State};
use axum::Json;
use axum::Router;
use nym_api_requests::models::{
    AllInclusionProbabilitiesResponse, InclusionProbabilityResponse, MixNodeBondAnnotated,
    MixnodeStatusResponse, StakeSaturationResponse,
};
use nym_mixnet_contract_common::MixId;
use serde::Deserialize;
use utoipa::IntoParams;

pub(super) fn mandatory_routes() -> Router<AxumAppState> {
    Router::new()
        .nest(
            "/mixnode/:mix_id",
            Router::new()
                .route("/status", axum::routing::get(get_mixnode_status))
                .route(
                    "/stake-saturation",
                    axum::routing::get(get_mixnode_stake_saturation),
                )
                .route(
                    "/inclusion-probability",
                    axum::routing::get(get_mixnode_inclusion_probability),
                ),
        )
        .merge(
            Router::new().nest(
                "/mixnodes",
                Router::new()
                    .route(
                        "/inclusion-probability",
                        axum::routing::get(get_mixnode_inclusion_probabilities),
                    )
                    .route("/detailed", axum::routing::get(get_mixnodes_detailed))
                    .route(
                        "/rewarded/detailed",
                        axum::routing::get(get_rewarded_set_detailed),
                    )
                    .route(
                        "/active/detailed",
                        axum::routing::get(get_active_set_detailed),
                    ),
            ),
        )
}

#[derive(Deserialize, IntoParams)]
struct MixIdParam {
    mix_id: MixId,
}

#[utoipa::path(
    tag = "status",
    get,
    params(
        MixIdParam
    ),
    path = "/v1/status/mixnode/{mix_id}/status",
    responses(
        (status = 200, body = MixnodeStatusResponse)
    )
)]
async fn get_mixnode_status(
    Path(MixIdParam { mix_id }): Path<MixIdParam>,
    State(state): State<AxumAppState>,
) -> Json<MixnodeStatusResponse> {
    Json(_get_mixnode_status(state.nym_contract_cache(), mix_id).await)
}

#[utoipa::path(
    tag = "status",
    get,
    params(
        MixIdParam
    ),
    path = "/v1/status/mixnode/{mix_id}/stake-saturation",
    responses(
        (status = 200, body = StakeSaturationResponse)
    )
)]
async fn get_mixnode_stake_saturation(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<StakeSaturationResponse>> {
    Ok(Json(
        _get_mixnode_stake_saturation(
            state.node_status_cache(),
            state.nym_contract_cache(),
            mix_id,
        )
        .await?,
    ))
}

#[utoipa::path(
    tag = "status",
    get,
    params(
        MixIdParam
    ),
    path = "/v1/status/mixnode/{mix_id}/inclusion-probability",
    responses(
        (status = 200, body = InclusionProbabilityResponse)
    )
)]
async fn get_mixnode_inclusion_probability(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<InclusionProbabilityResponse>> {
    Ok(Json(
        _get_mixnode_inclusion_probability(state.node_status_cache(), mix_id).await?,
    ))
}

#[utoipa::path(
    tag = "status",
    get,
    path = "/v1/status/mixnodes/inclusion-probability",
    responses(
        (status = 200, body = AllInclusionProbabilitiesResponse)
    )
)]
async fn get_mixnode_inclusion_probabilities(
    State(state): State<AxumAppState>,
) -> AxumResult<Json<AllInclusionProbabilitiesResponse>> {
    Ok(Json(
        _get_mixnode_inclusion_probabilities(state.node_status_cache()).await?,
    ))
}

#[utoipa::path(
    tag = "status",
    get,
    path = "/v1/status/mixnodes/detailed",
    responses(
        (status = 200, body = MixNodeBondAnnotated)
    )
)]
pub async fn get_mixnodes_detailed(
    State(state): State<AxumAppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_mixnodes_detailed(state.node_status_cache()).await)
}

#[utoipa::path(
    tag = "status",
    get,
    path = "/v1/status/mixnodes/rewarded/detailed",
    responses(
        (status = 200, body = MixNodeBondAnnotated)
    )
)]
pub async fn get_rewarded_set_detailed(
    State(state): State<AxumAppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_rewarded_set_detailed(state.node_status_cache()).await)
}

#[utoipa::path(
    tag = "status",
    get,
    path = "/v1/status/mixnodes/active/detailed",
    responses(
        (status = 200, body = MixNodeBondAnnotated)
    )
)]
pub async fn get_active_set_detailed(
    State(state): State<AxumAppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_active_set_detailed(state.node_status_cache()).await)
}
