// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Query, State},
    Router,
};
use nym_http_api_common::{FormattedResponse, OutputParams};
use tower_http::compression::CompressionLayer;

use crate::{
    http::state::AppState,
    models::{AvailableBandwidth, AxumErrorResponse, AxumResult},
};

pub(crate) fn bandwidth_routes() -> Router<AppState> {
    Router::new()
        .route("/available", axum::routing::get(available_bandwidth))
        .layer(CompressionLayer::new())
}

#[utoipa::path(
    tag = "bandwidth",
    get,
    path = "/v1/bandwidth/available",
    responses(
        (status = 200, content(
            (AvailableBandwidth = "application/bincode")
        ))

    ),
)]
#[axum::debug_handler]
async fn available_bandwidth(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<AvailableBandwidth>> {
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(
        state
            .available_bandwidth(addr.ip())
            .await
            .map_err(AxumErrorResponse::bad_request)?,
    ))
}
