// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::status::ApiStatusState;
use crate::support::http::state::AppState;
use axum::extract::State;
use axum::Json;
use axum::Router;
use nym_api_requests::models::{ApiHealthResponse, SignerInformationResponse};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_compact_ecash::Base58;

pub(crate) fn api_status_routes() -> Router<AppState> {
    Router::new()
        .route("/health", axum::routing::get(health))
        .route("/build-information", axum::routing::get(build_information))
        .route(
            "/signer-information",
            axum::routing::get(signer_information),
        )
}

#[utoipa::path(
    tag = "API Status",
    get,
    path = "/v1/api-status/health",
    responses(
        (status = 200, body = ApiHealthResponse)
    )
)]
async fn health(State(state): State<ApiStatusState>) -> Json<ApiHealthResponse> {
    let uptime = state.startup_time.elapsed();
    let health = ApiHealthResponse::new_healthy(uptime);
    Json(health)
}

#[utoipa::path(
    tag = "API Status",
    get,
    path = "/v1/api-status/build-information",
    responses(
        (status = 200, body = BinaryBuildInformationOwned)
    )
)]
async fn build_information(
    State(state): State<ApiStatusState>,
) -> Json<BinaryBuildInformationOwned> {
    Json(state.build_information.to_owned())
}

#[utoipa::path(
    tag = "API Status",
    get,
    path = "/v1/api-status/signer-information",
    responses(
        (status = 200, body = SignerInformationResponse)
    )
)]
async fn signer_information(
    State(state): State<ApiStatusState>,
) -> AxumResult<Json<SignerInformationResponse>> {
    let signer_state = state.signer_information.as_ref().ok_or_else(|| {
        AxumErrorResponse::internal_msg("this api does not expose zk-nym signing functionalities")
    })?;

    Ok(Json(SignerInformationResponse {
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
