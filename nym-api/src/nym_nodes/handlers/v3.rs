// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{ApiResult, AxumErrorResponse};
use crate::support::http::state::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::Router;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().nest("/stress-testing", stress_testing_routes())
}

fn stress_testing_routes() -> Router<AppState> {
    Router::new().route("/batch-submit", post(batch_submit_stress_testing_results))
}

#[utoipa::path(
    tag = "Nym Nodes",
    post,
    path = "/stress-testing/batch-submit",
    context_path = "/v3/nym-nodes",
    responses(
        (status = 501, description = "the endpoint has not been implemented yet"),
    ),
)]
async fn batch_submit_stress_testing_results(State(state): State<AppState>) -> ApiResult<()> {
    Err(AxumErrorResponse::not_implemented())
}
