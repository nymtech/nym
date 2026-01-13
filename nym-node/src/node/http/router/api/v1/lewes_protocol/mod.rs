// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::Router;
use axum::routing::get;
use nym_node_requests::api::v1::lewes_protocol::models;

pub mod root;

#[derive(Debug, Default, Clone, Copy)]
pub struct Config {
    pub details: Option<models::LewesProtocol>,
}

pub(crate) fn routes<S: Send + Sync + 'static + Clone>(config: Config) -> Router<S> {
    Router::new().route(
        "/",
        get({
            let lp_config = config.details;
            move |query| root::root_lewes_protocol(lp_config, query)
        }),
    )
}
