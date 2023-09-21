// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api::Output;
use axum::extract::Query;
use axum::response::IntoResponse;

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status=200, description ="", body = ())
    ),
    params(
        ("output" = Option<Output>, Query, description = "")
    )
)]
pub async fn build_info(Query(output): Query<Option<Output>>) -> impl IntoResponse {
    let output = output.unwrap_or_default();

    todo!()
}
