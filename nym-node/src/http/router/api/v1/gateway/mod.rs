// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::state::AppState;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;

#[derive(Debug, Clone, Default)]
pub struct Config {}

pub(crate) fn routes(_config: Config) -> Router<AppState> {
    Router::new().route("/", get(|| async { StatusCode::NOT_IMPLEMENTED }))
}
