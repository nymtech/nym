// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{ApiResult, AxumErrorResponse};
use crate::support::http::state::AppState;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_api_requests::models::network_monitor::KnownNetworkMonitorResponse;
use nym_crypto::asymmetric::ed25519;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().nest("/stress-testing", stress_testing_routes())
}

fn stress_testing_routes() -> Router<AppState> {
    Router::new()
        .route("/batch-submit", post(batch_submit_stress_testing_results))
        .route("/known-monitors/:identity_key", get(known_network_monitor))
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

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/stress-testing/known-monitors/{identity_key}",
    context_path = "/v3/nym-nodes",
    params(
        ("identity_key" = String, Path, description = "base58-encoded ed25519 identity key of the queried network monitor"),
    ),
    responses(
        (status = 200, body = KnownNetworkMonitorResponse),
        (status = 400, description = "the provided identity key is not a valid base58-encoded ed25519 public key"),
    ),
)]
async fn known_network_monitor(
    Path(identity_key): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<KnownNetworkMonitorResponse>> {
    let identity_key = ed25519::PublicKey::from_base58_string(&identity_key)
        .map_err(|err| AxumErrorResponse::bad_request(format!("malformed identity key: {err}")))?;

    let known = state
        .network_monitors()
        .get_or_refresh(&state.nyxd_client)
        .await?;

    let authorised = known.contains(&identity_key).await;

    Ok(Json(KnownNetworkMonitorResponse {
        identity_key,
        authorised,
    }))
}
