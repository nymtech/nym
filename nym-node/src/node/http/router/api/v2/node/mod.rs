// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::api::v2::node::auxiliary::auxiliary;
use crate::node::http::state::AppState;
use axum::Router;
use axum::routing::get;
use nym_node_requests::routes::api::v2;

pub mod auxiliary;

#[derive(Debug, Clone, Copy)]
pub struct Config {}

pub(super) fn routes(_config: Config) -> Router<AppState> {
    Router::new().route(v2::AUXILIARY, get(auxiliary))
}
