// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;

#[derive(Debug, Clone, Default)]
pub struct Config {}

pub(crate) fn routes(_config: Config) -> Router {
    Router::new().route("/", get(|| async { StatusCode::NOT_IMPLEMENTED }))
}
