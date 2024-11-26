// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_nodes::handlers::unstable::nym_node_routes_unstable;
use crate::support::http::state::AppState;
use axum::Router;

// as those get stabilised, they should get deprecated and use a redirection instead
pub(crate) fn unstable_routes() -> Router<AppState> {
    Router::new().nest("/nym-nodes", nym_node_routes_unstable())
}
