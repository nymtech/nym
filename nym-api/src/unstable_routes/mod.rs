// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(crate) mod account;
pub(crate) mod models;

use crate::support::http::state::AppState;
use axum::Router;

// as those get stabilised, they should get deprecated and use a redirection instead
pub(crate) fn unstable_routes() -> Router<AppState> {
    Router::new()
        .nest("/nym-nodes", crate::nym_nodes::handlers::unstable::routes())
        .nest("/account", account::routes())
}
