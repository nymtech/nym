// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network::models::NetworkDetailsV2;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Router;
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use tower_http::compression::CompressionLayer;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/details", axum::routing::get(network_details))
        .layer(CompressionLayer::new())
}

/// Identical to [`crate::network::handlers::v1::network_details`], except the returned `network` field uses the v2
/// (grouped `networking` block) version of the network details struct. This endpoint
/// is not a v2 of this API - it lives alongside `/details` under the same `/v1/network`
/// path - it's only the struct on the wire that changed shape.
#[utoipa::path(
    tag = "network",
    get,
    context_path = "/v2/network",
    path = "/details",
    responses(
        (status = 200, content(
            (NetworkDetailsV2 = "application/json"),
            (NetworkDetailsV2 = "application/yaml"),
        ))
    ),
    params(OutputParamsV2)
)]
async fn network_details(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> FormattedResponse<NetworkDetailsV2> {
    let output = output.output.unwrap_or_default();

    output.to_response(state.network_details().to_owned().into())
}
