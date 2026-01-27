// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::Query;
use axum::http::StatusCode;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::lewes_protocol::models::LewesProtocol;

/// Returns root Lewes Protocol information
#[utoipa::path(
    get,
    path = "",
    context_path = "/api/v1/lewes-protocol",
    tag = "Lewes Protocol",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            (LewesProtocol = "application/json"),
            (LewesProtocol = "application/yaml"),
            (LewesProtocol = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn root_lewes_protocol(
    config: Option<LewesProtocol>,
    Query(output): Query<OutputParams>,
) -> Result<LewesProtocolResponse, StatusCode> {
    let config = config.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    Ok(output.to_response(config))
}

pub type LewesProtocolResponse = FormattedResponse<LewesProtocol>;
