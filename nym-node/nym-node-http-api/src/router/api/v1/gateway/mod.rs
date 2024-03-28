// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::gateway::client_interfaces::wireguard::WireguardAppState;
use axum::routing::get;
use axum::Router;
use nym_node_requests::api::v1::gateway::models;
use nym_node_requests::routes::api::v1::gateway;

pub mod client_interfaces;
pub mod root;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<models::Gateway>,
}

pub(crate) fn routes<S: Send + Sync + 'static + Clone>(
    config: Config,
    initial_wg_state: WireguardAppState,
) -> Router<S> {
    Router::new()
        .route(
            "/",
            get({
                let gateway_details = config.details.clone();
                move |query| root::root_gateway(gateway_details, query)
            }),
        )
        .nest(
            gateway::CLIENT_INTERFACES,
            client_interfaces::routes(
                config.details.map(|g| g.client_interfaces),
                initial_wg_state,
            ),
        )
}
