// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::state::AppState;
use axum::routing::get;
use axum::Router;
use std::path::PathBuf;
use tower_http::services::ServeDir;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub assets_path: Option<PathBuf>,
}

pub(super) fn routes(config: Config) -> Router<AppState> {
    if let Some(assets) = config.assets_path {
        Router::new().nest_service("/", ServeDir::new(assets))
    } else {
        Router::new().route("/", get(default))
    }
}

pub(super) async fn default() -> &'static str {
    "default policy of the nym node"
}
