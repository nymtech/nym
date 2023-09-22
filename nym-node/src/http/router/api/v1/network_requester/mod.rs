// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::state::AppState;
use axum::routing::get;
use axum::Router;

pub mod root;
pub mod types;

pub(crate) mod routes {
    pub(crate) const ROOT: &str = "/";
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<types::NetworkRequester>,
}

pub(crate) fn routes(config: Config) -> Router<AppState> {
    Router::new().route(
        routes::ROOT,
        get({
            let network_requester_details = config.details;
            move |query| root::root_network_requester(network_requester_details, query)
        }),
    )
}
