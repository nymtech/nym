// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::middleware::auth::AuthLayer;
use crate::http::middleware::logging;
use crate::http::state::ApiState;
use axum::response::Redirect;
use axum::routing::{get, MethodRouter};
use axum::Router;
use nym_vpn_api_requests::routes;
use std::sync::Arc;
use zeroize::Zeroizing;

pub mod api;

fn swagger_redirect<S: Clone + Send + Sync + 'static>() -> MethodRouter<S> {
    // redirects with 303 status code
    get(|| async { Redirect::to("/api/v1/swagger/") })
}

pub fn build_router(state: ApiState, auth_token: String) -> Router {
    // let auth_layer = from_extractor::<RequireAuth>();
    let auth_middleware = AuthLayer::new(Arc::new(Zeroizing::new(auth_token)));

    let router = Router::new()
        // just redirect root and common typos for swagger for the current api version page (v1)
        .route("/", swagger_redirect())
        .route("/swagger", swagger_redirect())
        .route("/swagger/", swagger_redirect())
        .route("/swagger/index.html", swagger_redirect())
        .nest(routes::API, api::routes(auth_middleware))
        // we don't have to be using middleware, but we already had that code
        // we might want something like: https://github.com/tokio-rs/axum/blob/main/examples/tracing-aka-logging/src/main.rs#L44 instead
        .layer(axum::middleware::from_fn(logging::logger))
        .with_state(state);

    cfg_if::cfg_if! {
        if #[cfg(feature = "cors")] {
            router.layer(tower_http::cors::CorsLayer::very_permissive())
        } else {
            router
        }
    }
}
