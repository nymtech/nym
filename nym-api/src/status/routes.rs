// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::RocketErrorResponse;
use crate::status::ApiStatusState;
use nym_api_requests::models::{ApiHealthResponse, SignerInformationResponse};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_compact_ecash::Base58;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;

#[openapi(tag = "Api Status")]
#[get("/health")]
pub(crate) async fn health(state: &State<ApiStatusState>) -> Json<ApiHealthResponse> {
    let uptime = state.startup_time.elapsed();
    let health = ApiHealthResponse::new_healthy(uptime);
    Json(health)
}

#[openapi(tag = "Api Status")]
#[get("/build-information")]
pub(crate) async fn build_information(
    state: &State<ApiStatusState>,
) -> Json<BinaryBuildInformationOwned> {
    Json(state.build_information.to_owned())
}

#[openapi(tag = "Api Status")]
#[get("/signer-information")]
pub(crate) async fn signer_information(
    state: &State<ApiStatusState>,
) -> Result<Json<SignerInformationResponse>, RocketErrorResponse> {
    let signer_state = state.signer_information.as_ref().ok_or_else(|| {
        RocketErrorResponse::new(
            "this api does not expose zk-nym signing functionalities",
            Status::InternalServerError,
        )
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
