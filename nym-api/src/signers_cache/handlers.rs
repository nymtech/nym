// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::ApiResult;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::{
    DetailedSignersStatusResponse, DetailedSignersStatusResponseBody, SignersStatusOverview,
    SignersStatusResponse, SignersStatusResponseBody,
};
use nym_api_requests::signable::SignableMessageBody;
use nym_http_api_common::{FormattedResponse, OutputParams};

pub(crate) fn signers_routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(signers_status))
        .route("/status-detailed", get(signers_status_detailed))
}

#[utoipa::path(
    tag = "network",
    get,
    context_path = "/v1/network/signers",
    path = "/status",
    responses(
        (status = 200, content(
            (SignersStatusResponse = "application/json"),
            (SignersStatusResponse = "application/yaml"),
            (SignersStatusResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn signers_status(
    Query(params): Query<OutputParams>,
    State(state): State<AppState>,
) -> ApiResult<FormattedResponse<SignersStatusResponse>> {
    let output = params.get_output();

    let cached = state.ecash_signers_cache.get().await?;
    let as_at = cached.timestamp();
    Ok(output.to_response(
        SignersStatusResponseBody {
            as_at,
            overview: SignersStatusOverview::new(
                &cached.signers_results.results,
                cached.signers_results.threshold,
            ),
            results: cached
                .signers_results
                .results
                .iter()
                .map(Into::into)
                .collect(),
        }
        .sign(state.private_signing_key()),
    ))
}

#[utoipa::path(
    tag = "network",
    get,
    context_path = "/v1/network/signers",
    path = "/status-detailed",
    responses(
        (status = 200, content(
            (DetailedSignersStatusResponse = "application/json"),
            (DetailedSignersStatusResponse = "application/yaml"),
            (DetailedSignersStatusResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn signers_status_detailed(
    Query(params): Query<OutputParams>,
    State(state): State<AppState>,
) -> ApiResult<FormattedResponse<DetailedSignersStatusResponse>> {
    let output = params.get_output();

    let cached = state.ecash_signers_cache.get().await?;
    let as_at = cached.timestamp();
    Ok(output.to_response(
        DetailedSignersStatusResponseBody {
            as_at,
            overview: SignersStatusOverview::new(
                &cached.signers_results.results,
                cached.signers_results.threshold,
            ),
            details: cached.signers_results.results.clone(),
        }
        .sign(state.private_signing_key()),
    ))
}
