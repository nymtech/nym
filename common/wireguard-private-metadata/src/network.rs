// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Query, State},
    Json, Router,
};
use nym_http_api_common::{FormattedResponse, OutputParams};
use tower_http::compression::CompressionLayer;

use crate::{
    http::state::AppState,
    models::{
        latest::InnerTopUpRequest, AvailableBandwidthResponse, AxumErrorResponse, AxumResult,
        TopUpRequest,
    },
};

pub(crate) fn bandwidth_routes() -> Router<AppState> {
    Router::new()
        .route("/available", axum::routing::get(available_bandwidth))
        .route("/topup", axum::routing::post(topup_bandwidth))
        .layer(CompressionLayer::new())
}

#[utoipa::path(
    tag = "bandwidth",
    get,
    path = "/v1/bandwidth/available",
    responses(
        (status = 200, content(
            (AvailableBandwidthResponse = "application/bincode")
        ))

    ),
)]
async fn available_bandwidth(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<AvailableBandwidthResponse>> {
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(
        state
            .available_bandwidth(addr.ip())
            .await
            .map_err(AxumErrorResponse::bad_request)?,
    ))
}

#[utoipa::path(
    tag = "bandwidth",
    post,
    request_body = TopUpRequest,
    path = "/v1/bandwidth/topup",
    responses(
        (status = 200),
    ),
)]
async fn topup_bandwidth(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
    Json(request): Json<TopUpRequest>,
) -> AxumResult<FormattedResponse<()>> {
    let output = output.output.unwrap_or_default();
    let credential = InnerTopUpRequest::try_from(request)
        .map_err(AxumErrorResponse::bad_request)?
        .credential;
    state
        .topup_bandwidth(addr.ip(), credential)
        .await
        .map_err(AxumErrorResponse::bad_request)?;
    Ok(output.to_response(()))
}
