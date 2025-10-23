// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

use axum::{
    Json, Router,
    extract::{ConnectInfo, Query, State},
};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_wireguard_private_metadata_shared::{
    AxumErrorResponse, AxumResult, Construct, Extract, Request, Response, interface::RequestData,
    latest,
};
use tower_http::compression::CompressionLayer;

use crate::http::state::AppState;

pub(crate) fn bandwidth_routes() -> Router<AppState> {
    Router::new()
        .route("/version", axum::routing::get(version))
        .route("/available", axum::routing::post(available_bandwidth))
        .route("/topup", axum::routing::post(topup_bandwidth))
        .layer(CompressionLayer::new())
}

#[utoipa::path(
    tag = "bandwidth",
    get,
    path = "/v1/bandwidth/version",
    responses(
        (status = 200, content(
            (Response = "application/bincode")
        ))
    ),
)]
async fn version(Query(output): Query<OutputParams>) -> AxumResult<FormattedResponse<u64>> {
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(latest::VERSION.into()))
}

#[utoipa::path(
    tag = "bandwidth",
    post,
    request_body = Request,
    path = "/v1/bandwidth/available",
    responses(
        (status = 200, content(
            (Response = "application/bincode")
        ))
    ),
)]
async fn available_bandwidth(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
    Json(request): Json<Request>,
) -> AxumResult<FormattedResponse<Response>> {
    let output = output.output.unwrap_or_default();

    let (RequestData::AvailableBandwidth, version) =
        request.extract().map_err(AxumErrorResponse::bad_request)?
    else {
        return Err(AxumErrorResponse::bad_request("incorrect request type"));
    };
    let available_bandwidth_response = state
        .available_bandwidth(addr.ip())
        .await
        .map_err(AxumErrorResponse::bad_request)?;
    let response = Response::construct(available_bandwidth_response, version)
        .map_err(AxumErrorResponse::bad_request)?;

    Ok(output.to_response(response))
}

#[utoipa::path(
    tag = "bandwidth",
    post,
    request_body = Request,
    path = "/v1/bandwidth/topup",
    responses(
        (status = 200, content(
            (Response = "application/bincode")
        ))
    ),
)]
async fn topup_bandwidth(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
    Json(request): Json<Request>,
) -> AxumResult<FormattedResponse<Response>> {
    let output = output.output.unwrap_or_default();

    let (RequestData::TopUpBandwidth { credential }, version) =
        request.extract().map_err(AxumErrorResponse::bad_request)?
    else {
        return Err(AxumErrorResponse::bad_request("incorrect request type"));
    };
    let top_up_bandwidth_response = state
        .topup_bandwidth(addr.ip(), credential)
        .await
        .map_err(AxumErrorResponse::bad_request)?;
    let response = Response::construct(top_up_bandwidth_response, version)
        .map_err(AxumErrorResponse::bad_request)?;

    Ok(output.to_response(response))
}
