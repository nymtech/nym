// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::Router;

pub mod node;

#[derive(Debug, Clone)]
pub struct Config {
    pub node: node::Config,
}

pub(super) fn routes(config: Config) -> Router<AppState> {
    Router::new().merge(node::routes(config.node))
}
