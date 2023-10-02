// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::state::AppState;
use axum::Router;

pub mod gateway;
pub mod mixnode;
pub mod network_requester;
pub mod node;
pub mod openapi;

pub(crate) mod routes {
    pub(crate) const GATEWAY: &str = "/gateway";
    pub(crate) const MIXNODE: &str = "/mixnode";
    pub(crate) const NETWORK_REQUESTER: &str = "/network-requester";
    pub(crate) const SWAGGER: &str = "/swagger";
}

#[derive(Debug, Clone)]
pub struct Config {
    pub node: node::Config,
    pub gateway: gateway::Config,
    pub mixnode: mixnode::Config,
    pub network_requester: network_requester::Config,
}

pub(super) fn routes(config: Config) -> Router<AppState> {
    Router::new()
        .nest(routes::GATEWAY, gateway::routes(config.gateway))
        .nest(routes::MIXNODE, mixnode::routes(config.mixnode))
        .nest(
            routes::NETWORK_REQUESTER,
            network_requester::routes(config.network_requester),
        )
        .merge(node::routes(config.node))
        .merge(openapi::route())
}
