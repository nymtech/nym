// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::state::AppState;
use axum::Router;
use nym_node_requests::routes::api::v1;

pub mod gateway;
pub mod mixnode;
pub mod network_requester;
pub mod node;
pub mod openapi;

#[derive(Debug, Clone)]
pub struct Config {
    pub node: node::Config,
    pub gateway: gateway::Config,
    pub mixnode: mixnode::Config,
    pub network_requester: network_requester::Config,
}

pub(super) fn routes(config: Config) -> Router<AppState> {
    Router::new()
        .nest(v1::GATEWAY, gateway::routes(config.gateway))
        .nest(v1::MIXNODE, mixnode::routes(config.mixnode))
        .nest(
            v1::NETWORK_REQUESTER,
            network_requester::routes(config.network_requester),
        )
        .merge(node::routes(config.node))
        .merge(openapi::route())
}
