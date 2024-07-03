// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::AppState;
use axum::routing::get;
use axum::Router;
use nym_node_requests::routes::api::v1;

pub mod authenticator;
pub mod gateway;
pub mod health;
pub mod ip_packet_router;
pub mod metrics;
pub mod mixnode;
pub mod network_requester;
pub mod node;
pub mod openapi;

#[derive(Debug, Clone)]
pub struct Config {
    pub node: node::Config,
    pub metrics: metrics::Config,
    pub gateway: gateway::Config,
    pub mixnode: mixnode::Config,
    pub network_requester: network_requester::Config,
    pub ip_packet_router: ip_packet_router::Config,
    pub authenticator: authenticator::Config,
}

pub(super) fn routes(config: Config) -> Router<AppState> {
    Router::new()
        .route(v1::HEALTH, get(health::root_health))
        .nest(v1::METRICS, metrics::routes(config.metrics))
        .nest(v1::GATEWAY, gateway::routes(config.gateway))
        .nest(v1::MIXNODE, mixnode::routes(config.mixnode))
        .nest(
            v1::NETWORK_REQUESTER,
            network_requester::routes(config.network_requester),
        )
        .nest(
            v1::IP_PACKET_ROUTER,
            ip_packet_router::routes(config.ip_packet_router),
        )
        .nest(
            v1::AUTHENTICATOR,
            authenticator::routes(config.authenticator),
        )
        .merge(node::routes(config.node))
        .merge(openapi::route())
}
