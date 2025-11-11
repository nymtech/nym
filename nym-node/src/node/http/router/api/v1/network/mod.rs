// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::api::v1::network::upgrade_mode::upgrade_mode_status;
use crate::node::http::state::AppState;
use axum::Router;
use axum::routing::get;
use nym_node_requests::routes::api::v1::network;

pub mod upgrade_mode;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route(network::UPGRADE_MODE_STATUS, get(upgrade_mode_status))
}
