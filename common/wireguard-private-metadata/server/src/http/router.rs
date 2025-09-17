// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use core::net::SocketAddr;
use nym_http_api_common::middleware::logging::log_request_info;
use tokio::net::TcpListener;
use tokio_util::sync::WaitForCancellationFutureOwned;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::http::openapi::ApiDoc;
use crate::http::state::AppState;
use crate::network::bandwidth_routes;

/// Wrapper around `axum::Router` which ensures correct [order of layers][order].
/// Add new routes as if you were working directly with `axum`.
///
/// Why? Middleware like logger, CORS, TLS which need to handle request before other
/// layers should be added last. Using this builder pattern ensures that.
///
/// [order]: https://docs.rs/axum/latest/axum/middleware/index.html#ordering
pub struct RouterBuilder {
    unfinished_router: Router<AppState>,
}

impl RouterBuilder {
    /// All routes should be, if possible, added here. Exceptions are e.g.
    /// routes which are added conditionally in other places based on some `if`.
    pub fn with_default_routes() -> Self {
        let default_routes = Router::new()
            .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .route("/", get(|| async { Redirect::to("/swagger") }))
            .nest("/v1", Router::new().nest("/bandwidth", bandwidth_routes()));
        Self {
            unfinished_router: default_routes,
        }
    }

    /// Invoke this as late as possible before constructing HTTP server
    /// (after all routes were added).
    pub fn with_state(self, state: AppState) -> RouterWithState {
        RouterWithState {
            router: self.finalize_routes().with_state(state),
        }
    }

    /// Middleware added here intercepts the request before it gets to other routes.
    fn finalize_routes(self) -> Router<AppState> {
        self.unfinished_router
            .layer(setup_cors())
            .layer(axum::middleware::from_fn(log_request_info))
    }
}

fn setup_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers(tower_http::cors::Any)
        .allow_credentials(false)
}

pub struct RouterWithState {
    pub router: Router,
}

impl RouterWithState {
    pub async fn build_server(self, bind_address: &SocketAddr) -> anyhow::Result<ApiHttpServer> {
        let listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .map_err(|err| anyhow!("Couldn't bind to address {} due to {}", bind_address, err))?;

        Ok(ApiHttpServer {
            router: self.router,
            listener,
        })
    }
}

pub struct ApiHttpServer {
    router: Router,
    listener: TcpListener,
}

impl ApiHttpServer {
    pub async fn run(self, receiver: WaitForCancellationFutureOwned) -> Result<(), std::io::Error> {
        // into_make_service_with_connect_info allows us to see client ip address
        axum::serve(
            self.listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(receiver)
        .await
    }
}
