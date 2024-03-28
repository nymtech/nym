// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::routing::get;
use axum::Router;
use nym_node_requests::api::v1::ip_packet_router::models;

pub mod root;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<models::IpPacketRouter>,
}

pub(crate) fn routes<S: Send + Sync + 'static + Clone>(config: Config) -> Router<S> {
    Router::new().route(
        "/",
        get({
            let ip_packet_router_details = config.details;
            move |query| root::root_ip_packet_router(ip_packet_router_details, query)
        }),
    )
}
