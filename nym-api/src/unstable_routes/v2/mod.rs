// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::state::AppState;
use axum::Router;

pub(crate) mod nym_nodes;

pub(crate) fn unstable_routes_v2() -> Router<AppState> {
    Router::new().nest("/nym-nodes", nym_nodes::routes())
}
