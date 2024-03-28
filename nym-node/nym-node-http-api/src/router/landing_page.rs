// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::response::Html;
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

pub(super) async fn default() -> Html<&'static str> {
    Html(
        r#"
        <h1> Nym Node </h1>
        <div>
            <p> default page of the nym node - you can customize it by setting the 'assets' path under '[http]' section of your config. </p>

            You can explore the REST API at <a href = "/api/v1/swagger/">/api/v1/swagger/</a>
        </div>
    "#,
    )
}
