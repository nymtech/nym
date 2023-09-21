// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api::{FormattedResponse, OutputParams};
use crate::http::state::AppState;
use axum::extract::{Query, State};
use nym_bin_common::build_information::BinaryBuildInformationOwned;

/// Returns build metadata of the binary running the API
#[utoipa::path(
    get,
    path = "/build-info",
    context_path = "/api/v1",
    responses(
        (status = 200, content(
            ("application/json" = BinaryBuildInformationOwned),
            ("application/yaml" = BinaryBuildInformationOwned)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn build_info(
    State(state): State<AppState>,
    Query(output): Query<OutputParams>,
) -> BuildInfoResponse {
    let output = output.output.unwrap_or_default();
    // TODO: get rid of the clone since it's getting serialized anyway
    output.to_response(state.build_information.clone())
}

pub type BuildInfoResponse = FormattedResponse<BinaryBuildInformationOwned>;
