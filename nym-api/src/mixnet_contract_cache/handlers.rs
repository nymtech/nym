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
use nym_http_api_common::{FormattedResponse, OutputParams, OutputParamsV2};
use nym_mixnet_contract_common::{reward_params::RewardingParams, Interval};

// /v1/epoch/
pub(crate) fn epoch_routes() -> Router<AppState> {
    Router::new()
        .route("/reward_params", get(get_interval_reward_params))
        .route("/current", get(get_current_epoch))
        .route("/key-rotation-info", get(get_current_key_rotation_info))
}

// /v1/legacy
#[allow(deprecated)]
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
) -> AxumResult<FormattedResponse<Option<RewardingParams>>> {
    let output = output.output.unwrap_or_default();

    let snapshot = state.latest_snapshot().await?;
    Ok(output.to_response(Some(snapshot.mixnet().current_reward_params)))
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
) -> AxumResult<FormattedResponse<Option<Interval>>> {
    let output = output.output.unwrap_or_default();

    let snapshot = state.latest_snapshot().await?;
    Ok(output.to_response(Some(snapshot.mixnet().current_interval)))
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
        ))
    ),
    params(OutputParamsV2)
)]
async fn get_current_key_rotation_info(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> ApiResult<FormattedResponse<KeyRotationInfoResponse>> {
    let output = output.output.unwrap_or_default();

    let snapshot = state.latest_snapshot().await?;
    let mixnet = snapshot.mixnet();
    let details = KeyRotationDetails::new(mixnet.key_rotation_state, &mixnet.current_interval);

    Ok(output.to_response(details.into()))
}

/// Deprecated: legacy mixnodes are no longer read, so this is always empty.
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
#[deprecated]
async fn get_legacy_mixnodes(
    Query(output): Query<OutputParams>,
) -> FormattedResponse<LegacyMixnodesResponse> {
    output.get_output().to_response(LegacyMixnodesResponse {
        count: 0,
        nodes: Vec::new(),
    })
}

/// Deprecated: legacy gateways are no longer read, so this is always empty.
#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/gateways",
    context_path = "/v1/legacy",
    responses(
        (status = 200, content(
            (LegacyGatewaysResponse = "application/json"),
            (LegacyGatewaysResponse = "application/yaml"),
        ))
    ),
    params(OutputParamsV2)
)]
#[deprecated]
async fn get_legacy_gateways(
    Query(output): Query<OutputParamsV2>,
) -> FormattedResponse<LegacyGatewaysResponse> {
    output.get_output().to_response(LegacyGatewaysResponse {
        count: 0,
        nodes: Vec::new(),
    })
}
