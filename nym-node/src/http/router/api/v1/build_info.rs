// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api::{FormattedResponse, OutputParams};
use axum::extract::Query;
use nym_bin_common::build_information::BinaryBuildInformationOwned;

/// Returns build metadata of the binary running the API
#[utoipa::path(
    get,
    path = "/build-info",
    context_path = "/api/v1",
    tag = "Base",
    responses(
        (status = 200, content(
            ("application/json" = BinaryBuildInformationOwned),
            ("application/yaml" = BinaryBuildInformationOwned)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn build_info(
    build_information: BinaryBuildInformationOwned,
    Query(output): Query<OutputParams>,
) -> BuildInfoResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(build_information)
}

pub type BuildInfoResponse = FormattedResponse<BinaryBuildInformationOwned>;
