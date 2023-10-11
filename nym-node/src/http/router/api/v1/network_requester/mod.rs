// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::routing::get;
use axum::Router;
use nym_node_requests::api::v1::network_requester::models;

pub mod root;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<models::NetworkRequester>,
}

pub(crate) fn routes<S: Send + Sync + 'static + Clone>(config: Config) -> Router<S> {
    Router::new().route(
        "/",
        get({
            let network_requester_details = config.details;
            move |query| root::root_network_requester(network_requester_details, query)
        }),
    )
}
