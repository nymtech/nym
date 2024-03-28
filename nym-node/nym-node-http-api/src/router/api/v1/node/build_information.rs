// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::router::api::{FormattedResponse, OutputParams};
use axum::extract::Query;
use nym_node_requests::api::v1::node::models::BinaryBuildInformationOwned;

/// Returns build metadata of the binary running the API
#[utoipa::path(
    get,
    path = "/build-information",
    context_path = "/api/v1",
    tag = "Node",
    responses(
        (status = 200, content(
            ("application/json" = BinaryBuildInformationOwned),
            ("application/yaml" = BinaryBuildInformationOwned)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn build_information(
    build_information: BinaryBuildInformationOwned,
    Query(output): Query<OutputParams>,
) -> BuildInformationResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(build_information)
}

pub type BuildInformationResponse = FormattedResponse<BinaryBuildInformationOwned>;
