// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::state::AppState;
use axum::routing::get;
use axum::Router;

pub mod root;

pub(crate) mod routes {
    pub(crate) const ROOT: &str = "/";
}

#[derive(Debug, Clone, Default)]
pub struct Config {}

pub(crate) fn routes(_config: Config) -> Router<AppState> {
    Router::new().route(routes::ROOT, get(root::root_gateway))
}
