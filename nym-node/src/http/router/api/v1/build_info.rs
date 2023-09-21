// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api::{FormattedResponse, OutputParams};
use crate::http::state::AppState;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use nym_bin_common::build_information::BinaryBuildInformationOwned;

/// Description of the path
#[utoipa::path(
    get,
    path = "/build-info",
    context_path = "/api/v1",
    responses(
        (status=200, content(
            ("application/json" = BinaryBuildInformationOwned),
            ("application/yaml" = BinaryBuildInformationOwned)
        ))
    ),
    params(OutputParams)
)]
pub async fn build_info(
    State(state): State<AppState>,
    Query(output): Query<OutputParams>,
) -> FormattedResponse<BinaryBuildInformationOwned> {
    let output = output.output.unwrap_or_default();
    // todo!()
    // TODO: get rid of the clone since it's getting serialized anyway
    output.to_response(state.build_information.clone())
}

pub struct BuildInfoResponse(BinaryBuildInformationOwned);
