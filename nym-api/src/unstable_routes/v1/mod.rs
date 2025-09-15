// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::state::AppState;
use axum::Router;

pub(crate) mod account;
pub(crate) mod nym_nodes;

// as those get stabilised, they should get deprecated and use a redirection instead
pub(crate) fn unstable_routes_v1() -> Router<AppState> {
    Router::new()
        .nest("/nym-nodes", nym_nodes::routes())
        .nest("/account", account::routes())
}
