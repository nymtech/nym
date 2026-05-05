// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AppState;
use axum::Router;
use axum::response::Redirect;
use axum::routing::{MethodRouter, get};
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_http_api_common::middleware::logging::log_request_debug;
use nym_network_monitor_orchestrator_requests::routes;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};
use zeroize::Zeroizing;

pub(crate) mod api_docs;
pub(crate) mod v1;

/// Returns a handler that issues a 303 redirect to the Swagger UI.
fn swagger_redirect<S: Clone + Send + Sync + 'static>() -> MethodRouter<S> {
    // redirects with 303 status code
    get(|| async { Redirect::to(routes::SWAGGER) })
}

/// Assembles the full orchestrator HTTP router with Swagger UI, v1 API routes,
/// bearer-auth middleware, and request logging.
pub(crate) fn build_router(
    state: AppState,
    agents_auth_token: Arc<Zeroizing<String>>,
    metrics_and_results_auth_token: Arc<Zeroizing<String>>,
) -> Router {
    let agents_auth = AuthLayer::new(agents_auth_token);
    let metrics_and_results_auth = AuthLayer::new(metrics_and_results_auth_token);

    Router::new()
        .route(routes::ROOT, swagger_redirect())
        .route("/swagger/index.html", swagger_redirect())
        .merge(api_docs::route())
        .nest(
            routes::V1,
            v1::routes(agents_auth, metrics_and_results_auth),
        )
        .layer(axum::middleware::from_fn(log_request_debug))
        .with_state(state)
}

/// Binds to `bind_address` and serves the given router until the shutdown token is cancelled.
/// The listener is created with `into_make_service_with_connect_info` so handlers can
/// extract the peer [`SocketAddr`].
pub(crate) async fn run_http_server(
    router: Router,
    bind_address: SocketAddr,
    shutdown_token: ShutdownToken,
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(bind_address)
        .await
        .inspect_err(|err| error!("couldn't bind to address {bind_address}: {err}"))?;

    info!("starting http api server on {bind_address}");

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move { shutdown_token.cancelled().await })
    .await?;

    Ok(())
}
