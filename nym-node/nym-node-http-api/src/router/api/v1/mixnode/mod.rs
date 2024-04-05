// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::routing::get;
use axum::Router;
use nym_node_requests::api::v1::mixnode::models;

pub mod root;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<models::Mixnode>,
}

pub(crate) fn routes<S: Send + Sync + 'static + Clone>(config: Config) -> Router<S> {
    Router::new().route(
        "/",
        get({
            let mixnode_details = config.details;
            move |query| root::root_mixnode(mixnode_details, query)
        }),
    )
}
