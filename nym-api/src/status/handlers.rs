// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::status::ApiStatusState;
use crate::support::config::CHAIN_STALL_THRESHOLD;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::{Json, Router};
use nym_api_requests::models::{
    ApiHealthResponse, ApiInformationResponse, ApiStatus, ChainStatus, KeyPossessionChallenge,
    KeyPossessionChallengeResponse, SignerInformationResponse,
};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_compact_ecash::Base58;
use nym_http_api_common::{FormattedResponse, OutputParams, OutputParamsV2};
use time::OffsetDateTime;

pub(crate) fn api_status_routes() -> Router<AppState> {
    Router::new()
        .route("/health", axum::routing::get(health))
        .route("/build-information", axum::routing::get(build_information))
        .route(
            "/signer-information",
            axum::routing::get(signer_information),
        )
        .route("/api-information", axum::routing::get(api_information))
        .route(
            "/prove_key_possession",
            axum::routing::post(prove_key_possession),
        )
}

#[utoipa::path(
    tag = "API Status",
    get,
    path = "/v1/api-status/health",
    responses(
        (status = 200, content(
            (ApiHealthResponse = "application/json"),
            (ApiHealthResponse = "application/yaml"),
            (ApiHealthResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn health(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<ApiHealthResponse> {
    let output = output.output.unwrap_or_default();

    let uptime = state.api_status.startup_time.elapsed();
    let chain_status = match state
        .chain_status_cache
        .get_or_refresh(&state.nyxd_client)
        .await
    {
        Ok(res) => {
            let now = OffsetDateTime::now_utc();
            res.stall_status(now, CHAIN_STALL_THRESHOLD)
        }
        Err(_) => ChainStatus::Unknown,
    };
    let health = ApiHealthResponse {
        status: ApiStatus::Up,
        chain_status,
        uptime: uptime.as_secs(),
    };
    output.to_response(health)
}

#[utoipa::path(
    tag = "API Status",
    get,
    path = "/v1/api-status/build-information",
    responses(
        (status = 200, content(
            (BinaryBuildInformationOwned = "application/json"),
            (BinaryBuildInformationOwned = "application/yaml"),
            (BinaryBuildInformationOwned = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn build_information(
    Query(output): Query<OutputParams>,
    State(state): State<ApiStatusState>,
) -> FormattedResponse<BinaryBuildInformationOwned> {
    let output = output.output.unwrap_or_default();

    output.to_response(state.build_information.to_owned())
}

#[utoipa::path(
    tag = "API Status",
    get,
    path = "/v1/api-status/signer-information",
    responses(
        (status = 200, content(
            (SignerInformationResponse = "application/json"),
            (SignerInformationResponse = "application/yaml"),
            (SignerInformationResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn signer_information(
    Query(output): Query<OutputParams>,
    State(state): State<ApiStatusState>,
) -> AxumResult<FormattedResponse<SignerInformationResponse>> {
    let signer_state = state.signer_information.as_ref().ok_or_else(|| {
        AxumErrorResponse::internal_msg("this api does not expose zk-nym signing functionalities")
    })?;

    let output = output.output.unwrap_or_default();

    Ok(output.to_response(SignerInformationResponse {
        cosmos_address: signer_state.cosmos_address.clone(),
        identity: signer_state.identity.clone(),
        announce_address: signer_state.announce_address.clone(),
        verification_key: signer_state
            .ecash_keypair
            .verification_key()
            .await
            .map(|maybe_vk| maybe_vk.to_bs58()),
    }))
}

#[utoipa::path(
    tag = "API Status",
    get,
    path = "/api-information",
    context_path = "/v1/api-status",
    responses(
        (status = 200, content(
            (ApiInformationResponse = "application/json"),
            (ApiInformationResponse = "application/yaml"),
        ))
    ),
    params(
        OutputParamsV2,
    )
)]
async fn api_information(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> FormattedResponse<ApiInformationResponse> {
    // we only allow json and yaml responses so we can easily add additional fields later
    output.to_response(ApiInformationResponse {
        identity: *state.identity_keypair.public_key(),
    })
}

#[utoipa::path(
    tag = "API Status",
    post,
    path = "/prove-key-possession",
    context_path = "/v1/api-status",
    responses(
        (status = 200, content(
            (KeyPossessionChallengeResponse = "application/json"),
            (KeyPossessionChallengeResponse = "application/yaml"),
        ))
    ),
    params(
        OutputParamsV2,
    )
)]
async fn prove_key_possession(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
    Json(challenge): Json<KeyPossessionChallenge>,
) -> FormattedResponse<KeyPossessionChallengeResponse> {
    output.to_response(challenge.sign(state.identity_keypair.private_key()))
}
