// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::models::status::{
    ApiStatus, HealthResponse, PriceScraperLastError, PriceScraperLastSuccess,
    PriceScraperStatusResponse,
};
use crate::http::state::{AppState, PriceScraperState, StatusState};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use nym_bin_common::build_information::BinaryBuildInformationOwned;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/build-information", get(build_information))
        .route("/price-scraper", get(price_scraper_status))
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/build-information",
    context_path = "/v1/status",
    responses(
        (status = 200, body = BinaryBuildInformationOwned)
    )
)]
async fn build_information(State(state): State<StatusState>) -> Json<BinaryBuildInformationOwned> {
    Json(state.build_information.to_owned())
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/health",
    context_path = "/v1/status",
    responses(
        (status = 200, body = HealthResponse)
    )
)]
async fn health(State(state): State<StatusState>) -> Json<HealthResponse> {
    let uptime = state.startup_time.elapsed();

    let health = HealthResponse {
        status: ApiStatus::Up,
        uptime: uptime.as_secs(),
    };
    Json(health)
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/price-scraper",
    context_path = "/v1/status",
    responses(
        (status = 200, body = PriceScraperStatusResponse)
    )
)]
pub(crate) async fn price_scraper_status(
    State(state): State<PriceScraperState>,
) -> Json<PriceScraperStatusResponse> {
    let guard = state.inner.read().await;
    Json(PriceScraperStatusResponse {
        last_success: guard
            .last_success
            .as_ref()
            .map(|s| PriceScraperLastSuccess {
                timestamp: s.timestamp,
                response: s.response.clone(),
            }),
        last_failure: guard.last_failure.as_ref().map(|f| PriceScraperLastError {
            timestamp: f.timestamp,
            message: f.message.clone(),
        }),
    })
}
