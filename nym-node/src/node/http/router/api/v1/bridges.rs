// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::Router;
use nym_node_requests::api::v1::gateway::models;
use tower_http::services::ServeFile;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub details: Option<models::Bridges>,
}

pub(crate) fn routes<S: Send + Sync + 'static + Clone>(config: Config) -> Router<S> {
    if let Some(cfg) = config.details {
        Router::new().route_service("/client-params", ServeFile::new(cfg.client_params_path))
    } else {
        Router::new()
    }
}
