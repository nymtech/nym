// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::routing::get;
use axum::Router;
use std::path::PathBuf;
use tower_http::services::ServeDir;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub assets_path: Option<PathBuf>,
}

pub(super) fn routes<S: Send + Sync + 'static + Clone>(config: Config) -> Router<S> {
    if let Some(assets) = config.assets_path {
        Router::new().nest_service("/", ServeDir::new(assets))
    } else {
        Router::new().route("/", get(default))
    }
}

pub(super) async fn default() -> &'static str {
    "default page of the nym node"
}
