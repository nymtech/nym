// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::{SignedLewesProtocol, SignedLewesProtocolInfo};

/// Returns root Lewes Protocol information
#[utoipa::path(
    get,
    path = "/lewes-protocol",
    context_path = "/api/v1",
    tag = "v1 / Lewes Protocol",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            (SignedLewesProtocolInfo = "application/json"),
            (SignedLewesProtocolInfo = "application/yaml"),
            (SignedLewesProtocolInfo = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn root_lewes_protocol(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> Result<LewesProtocolResponse, StatusCode> {
    Ok(output.to_response(state.static_information.lewes_protocol.clone()))
}

pub type LewesProtocolResponse = FormattedResponse<SignedLewesProtocol>;
