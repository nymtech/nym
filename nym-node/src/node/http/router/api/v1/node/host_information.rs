// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::Query;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::{v1::node::models::SignedHostInformation, SignedDataHostInfo};

/// Returns host information of this node.
#[utoipa::path(
    get,
    path = "/host-information",
    context_path = "/api/v1",
    tag = "Node",
    responses(
        (status = 200, content(
            (SignedDataHostInfo = "application/json"),
            (SignedDataHostInfo = "application/yaml")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn host_information(
    host_information: SignedHostInformation,
    Query(output): Query<OutputParams>,
) -> HostInformationResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(host_information)
}

pub type HostInformationResponse = FormattedResponse<SignedHostInformation>;
