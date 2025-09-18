// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{ApiResult, AxumResult};
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::{
    KeyRotationDetails, KeyRotationInfoResponse, LegacyGatewaysResponse, LegacyMixnodesResponse,
};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_mixnet_contract_common::{reward_params::RewardingParams, Interval};

// /v1/epoch/
pub(crate) fn epoch_routes() -> Router<AppState> {
    Router::new()
        .route("/reward_params", get(get_interval_reward_params))
        .route("/current", get(get_current_epoch))
        .route("/key-rotation-info", get(get_current_key_rotation_info))
}

// /v1/legacy
pub(crate) fn legacy_nodes_routes() -> Router<AppState> {
    Router::new()
        .route("/mixnodes", get(get_legacy_mixnodes))
        .route("/gateways", get(get_legacy_gateways))
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/epoch/reward_params",
    responses(
        (status = 200, content(
            (Option<RewardingParams> = "application/json"),
            (Option<RewardingParams> = "application/yaml"),
            (Option<RewardingParams> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_interval_reward_params(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Option<RewardingParams>> {
    let output = output.output.unwrap_or_default();

    output.to_response(
        state
            .nym_contract_cache()
            .interval_reward_params()
            .await
            .ok(),
    )
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/epoch/current",
    responses(
        (status = 200, content(
            (Option<Interval> = "application/json"),
            (Option<Interval> = "application/yaml"),
            (Option<Interval> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_current_epoch(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Option<Interval>> {
    let output = output.output.unwrap_or_default();

    output.to_response(state.nym_contract_cache().current_interval().await.ok())
}

//
#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/key-rotation-info",
    context_path = "/v1/epoch",
    responses(
        (status = 200, content(
            (KeyRotationInfoResponse = "application/json"),
            (KeyRotationInfoResponse = "application/yaml"),
            (KeyRotationInfoResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_current_key_rotation_info(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> ApiResult<FormattedResponse<KeyRotationInfoResponse>> {
    let output = output.output.unwrap_or_default();

    let contract_cache = state.nym_contract_cache();
    let current_interval = contract_cache.current_interval().await?;
    let key_rotation_state = contract_cache.get_key_rotation_state().await?;

    let details = KeyRotationDetails {
        key_rotation_state,
        current_absolute_epoch_id: current_interval.current_epoch_absolute_id(),
        current_epoch_start: current_interval.current_epoch_start(),
        epoch_duration: current_interval.epoch_length(),
    };

    Ok(output.to_response(details.into()))
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/mixnodes",
    context_path = "/v1/legacy",
    responses(
        (status = 200, content(
            (LegacyMixnodesResponse = "application/json"),
            (LegacyMixnodesResponse = "application/yaml"),
            (LegacyMixnodesResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_legacy_mixnodes(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<LegacyMixnodesResponse>> {
    let output = output.get_output();
    let contract_cache = state.nym_contract_cache();
    let mixnodes = contract_cache.legacy_mixnodes().await?;
    Ok(output.to_response(LegacyMixnodesResponse {
        count: mixnodes.len(),
        nodes: mixnodes,
    }))
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/gateways",
    context_path = "/v1/legacy",
    responses(
        (status = 200, content(
            (LegacyGatewaysResponse = "application/json"),
            (LegacyGatewaysResponse = "application/yaml"),
            (LegacyGatewaysResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_legacy_gateways(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<LegacyGatewaysResponse>> {
    let output = output.get_output();
    let contract_cache = state.nym_contract_cache();
    let gateways = contract_cache.legacy_gateways().await?;
    Ok(output.to_response(LegacyGatewaysResponse {
        count: gateways.len(),
        nodes: gateways,
    }))
}
