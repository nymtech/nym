// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::state::AppState;
use axum::routing::get;
use axum::Router;
use nym_node_requests::api::v1::mixnode::models;

pub mod root;

pub(crate) mod routes {
    pub(crate) const ROOT: &str = "/";
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<models::Mixnode>,
}

pub(crate) fn routes(config: Config) -> Router<AppState> {
    Router::new().route(
        routes::ROOT,
        get({
            let mixnode_details = config.details;
            move |query| root::root_mixnode(mixnode_details, query)
        }),
    )
}
