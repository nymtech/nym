// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api::Output;
use axum::extract::Query;
use axum::response::IntoResponse;

pub async fn build_info(output: Query<Option<Output>>) -> impl IntoResponse {
    let output = output.unwrap_or_default();

    todo!()
}
