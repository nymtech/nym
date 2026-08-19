// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::router::api::openapi;
use crate::http::state::AppState;
use anyhow::bail;
use axum::Router;
use axum::response::Redirect;
use axum::routing::{MethodRouter, get};
use nym_geolocator_requests::routes;
use nym_geolocator_requests::routes::API;
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_http_api_common::middleware::logging::log_request_debug;
use std::sync::Arc;
use zeroize::Zeroizing;

pub(crate) mod api;

/// Returns a handler that issues a 303 redirect to the Swagger UI.
fn swagger_redirect<S: Clone + Send + Sync + 'static>() -> MethodRouter<S> {
    // redirects with 303 status code
    get(|| async { Redirect::to(routes::SWAGGER) })
}

pub(crate) fn build_router(state: AppState, auth_token: String) -> anyhow::Result<Router> {
    if auth_token.is_empty() {
        bail!("can't use empty auth token")
    }

    let auth_middleware = AuthLayer::new(Arc::new(Zeroizing::new(auth_token)));

    Ok(Router::new()
        .route(routes::ROOT, swagger_redirect())
        .merge(openapi::route())
        .nest(API, api::routes(auth_middleware))
        .layer(axum::middleware::from_fn(log_request_debug))
        .with_state(state))
}
