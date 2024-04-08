// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::ErrorResponse;
use crate::status::ApiStatusState;
use nym_api_requests::models::{ApiHealthResponse, SignerInformation};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
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
) -> Result<Json<SignerInformation>, ErrorResponse> {
    state
        .signer_information
        .clone()
        .ok_or_else(|| {
            ErrorResponse::new(
                "this api does not expose zk-nym signing functionalities",
                Status::InternalServerError,
            )
        })
        .map(Json)
}
