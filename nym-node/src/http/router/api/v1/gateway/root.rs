// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymNodeError;
use crate::http::router::api::OutputParams;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;

#[utoipa::path(
    get,
    path = "",
    context_path = "/api/v1/gateway",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet")
    ),
    params(OutputParams)
)]
pub(crate) async fn root_gateway(Query(_output): Query<OutputParams>) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        NymNodeError::Unimplemented.to_string(),
    )
}
