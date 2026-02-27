// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::Router;
use axum::routing::get;
use nym_node_requests::api::SignedLewesProtocol;

pub mod root;

#[derive(Debug, Clone)]
pub struct Config {
    pub details: SignedLewesProtocol,
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
