// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::Router;
use axum::routing::get;

pub mod root;

#[derive(Debug, Clone, Default)]
pub struct Config {}

pub(crate) fn routes(_config: Config) -> Router<AppState> {
    Router::new().route("/", get(root::root_lewes_protocol))
}
