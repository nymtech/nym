// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::gateway::client_interfaces::wireguard::WireguardAppState;
use crate::state::AppState;
use axum::Router;
use nym_node_requests::routes;

pub mod v1;

pub(crate) use nym_http_api_common::{FormattedResponse, Output, OutputParams};

pub use nym_node_requests::api as api_requests;

#[derive(Debug, Clone)]
pub struct Config {
    pub v1_config: v1::Config,
}

pub(super) fn routes(config: Config, initial_wg_state: WireguardAppState) -> Router<AppState> {
    Router::new().nest(
        routes::api::V1,
        v1::routes(config.v1_config, initial_wg_state),
    )
}
