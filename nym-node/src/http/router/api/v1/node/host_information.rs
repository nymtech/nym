// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::api::v1::node::types::HostInformation;
use crate::http::api::{FormattedResponse, OutputParams};
use axum::extract::Query;

/// Returns host information of this node.
#[utoipa::path(
    get,
    path = "/host-information",
    context_path = "/api/v1",
    tag = "Node",
    responses(
        (status = 200, content(
            ("application/json" = HostInformation),
            ("application/yaml" = HostInformation)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn host_information(
    host_information: HostInformation,
    Query(output): Query<OutputParams>,
) -> HostInformationResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(host_information)
}

pub type HostInformationResponse = FormattedResponse<HostInformation>;
