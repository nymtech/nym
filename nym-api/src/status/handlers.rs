// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::status::ApiStatusState;
use crate::{support::http::static_routes, v2::AxumAppState};
use axum::Json;
use axum::Router;
use nym_api_requests::models::{ApiHealthResponse, SignerInformationResponse};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_compact_ecash::Base58;
use std::sync::Arc;

pub(crate) fn api_status_routes() -> Router<AxumAppState> {
    let api_status_state = Arc::new(ApiStatusState::new());

    Router::new()
        .route(
            &static_routes::v1::api_status::health(),
            axum::routing::get({
                let state = Arc::clone(&api_status_state);
                || health(state)
            }),
        )
        .route(
            &static_routes::v1::api_status::build_information(),
            axum::routing::get({
                let state = Arc::clone(&api_status_state);
                || build_information(state)
            }),
        )
        .route(
            &static_routes::v1::api_status::signer_information(),
            axum::routing::get({
                let state = Arc::clone(&api_status_state);
                || signer_information(state)
            }),
        )
}

async fn health(state: Arc<ApiStatusState>) -> Json<ApiHealthResponse> {
    let uptime = state.startup_time.elapsed();
    let health = ApiHealthResponse::new_healthy(uptime);
    Json(health)
}

async fn build_information(state: Arc<ApiStatusState>) -> Json<BinaryBuildInformationOwned> {
    Json(state.build_information.to_owned())
}

async fn signer_information(
    state: Arc<ApiStatusState>,
) -> AxumResult<Json<SignerInformationResponse>> {
    let signer_state = state.signer_information.as_ref().ok_or_else(|| {
        // TODO dz this shouldn't be returned to client OR shouldn't be a 500: possible 405 ?
        AxumErrorResponse::internal_msg("this api does not expose zk-nym signing functionalities")
    })?;

    Ok(Json(SignerInformationResponse {
        cosmos_address: signer_state.cosmos_address.clone(),
        identity: signer_state.identity.clone(),
        announce_address: signer_state.announce_address.clone(),
        verification_key: signer_state
            .coconut_keypair
            .verification_key()
            .await
            .map(|maybe_vk| maybe_vk.to_bs58()),
    }))
}
