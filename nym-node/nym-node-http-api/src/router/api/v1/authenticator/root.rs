// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::router::api::{FormattedResponse, OutputParams};
use axum::extract::Query;
use axum::http::StatusCode;
use nym_node_requests::api::v1::authenticator::models::Authenticator;

/// Returns root authenticator information
#[utoipa::path(
    get,
    path = "",
    context_path = "/api/v1/authenticator",
    tag = "Authenticator",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            ("application/json" = Authenticator),
            ("application/yaml" = Authenticator)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn root_authenticator(
    details: Option<Authenticator>,
    Query(output): Query<OutputParams>,
) -> Result<AuthenticatorResponse, StatusCode> {
    let details = details.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(details))
}

pub type AuthenticatorResponse = FormattedResponse<Authenticator>;
