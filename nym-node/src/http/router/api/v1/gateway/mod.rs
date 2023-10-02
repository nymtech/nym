// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::state::AppState;
use axum::routing::get;
use axum::Router;

pub mod client_interfaces;
pub mod root;
pub mod types;

pub(crate) mod routes {
    pub(crate) const ROOT: &str = "/";
    pub(crate) const CLIENT_INTERFACES: &str = "/client-interfaces";
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<types::Gateway>,
}

pub(crate) fn routes(config: Config) -> Router<AppState> {
    Router::new()
        .route(
            routes::ROOT,
            get({
                let gateway_details = config.details.clone();
                move |query| root::root_gateway(gateway_details, query)
            }),
        )
        .nest(
            routes::CLIENT_INTERFACES,
            client_interfaces::routes(config.details.map(|g| g.client_interfaces)),
        )
}
