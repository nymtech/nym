// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::node::models::BinaryBuildInformationOwned;

/// Returns build metadata of the binary running the API
#[utoipa::path(
    get,
    path = "/build-information",
    context_path = "/api/v1",
    tag = "v1 / Node",
    responses(
        (status = 200, content(
            (BinaryBuildInformationOwned = "application/json"),
            (BinaryBuildInformationOwned = "application/yaml")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn build_information(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> BuildInformationResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(state.static_information.build_information.clone())
}

pub type BuildInformationResponse = FormattedResponse<BinaryBuildInformationOwned>;
