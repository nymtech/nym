// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::network_requester::exit_policy::node_exit_policy;
use axum::routing::get;
use axum::Router;
use nym_node_requests::api::v1::network_requester::exit_policy::models::UsedExitPolicy;
use nym_node_requests::api::v1::network_requester::models;
use nym_node_requests::routes::api::v1::network_requester;

pub mod exit_policy;
pub mod root;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<models::NetworkRequester>,
    pub exit_policy: Option<UsedExitPolicy>,
}

pub(crate) fn routes<S: Send + Sync + 'static + Clone>(config: Config) -> Router<S> {
    Router::new()
        .route(
            "/",
            get({
                let network_requester_details = config.details;
                move |query| root::root_network_requester(network_requester_details, query)
            }),
        )
        .route(
            network_requester::EXIT_POLICY,
            get({
                let policy = config.exit_policy.unwrap_or_default();
                move |query| node_exit_policy(policy, query)
            }),
        )
}
