// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::circulating_supply_api::handlers::circulating_supply_routes;
use crate::network::handlers::nym_network_routes;
use crate::node_status_api::handlers::node_status_routes;
use crate::nym_contract_cache::handlers::nym_contract_cache_routes;
use crate::nym_nodes::handlers::legacy::legacy_nym_node_routes;
use crate::nym_nodes::handlers::nym_node_routes;
use crate::status;
use crate::support::http::openapi::ApiDoc;
use crate::support::http::state::AppState;
use crate::support::http::unstable_routes::unstable_routes;
use anyhow::anyhow;
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use core::net::SocketAddr;
use nym_http_api_common::logging::logger;
use tokio::net::TcpListener;
use tokio_util::sync::WaitForCancellationFutureOwned;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Wrapper around `axum::Router` which ensures correct [order of layers][order].
/// Add new routes as if you were working directly with `axum`.
///
/// Why? Middleware like logger, CORS, TLS which need to handle request before other
/// layers should be added last. Using this builder pattern ensures that.
///
/// [order]: https://docs.rs/axum/latest/axum/middleware/index.html#ordering
pub(crate) struct RouterBuilder {
    unfinished_router: Router<AppState>,
}

impl RouterBuilder {
    /// All routes should be, if possible, added here. Exceptions are e.g.
    /// routes which are added conditionally in other places based on some `if`.
    pub(crate) fn with_default_routes(network_monitor: bool) -> Self {
        // https://docs.rs/tower-http/0.1.1/tower_http/trace/index.html
        // TODO rocket use tracing instead of env_logger
        // https://github.com/tokio-rs/axum/blob/main/examples/tracing-aka-logging/src/main.rs
        // .layer(
        //     TraceLayer::new_for_http()
        //         .make_span_with(DefaultMakeSpan::new().include_headers(true))
        //         .on_request(DefaultOnRequest::new())
        //         .on_response(DefaultOnResponse::new().latency_unit(tower_http::LatencyUnit::Micros)),
        // )
        // .route("/swagger", axum::routing::get(hello))
        let default_routes = Router::new()
            .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .route("/", get(|| async { Redirect::to("/swagger") }))
            .nest(
                "/v1",
                Router::new()
                    // unfortunately some routes didn't use correct prefix and were attached to the root
                    .merge(nym_contract_cache_routes())
                    .merge(legacy_nym_node_routes())
                    .nest("/circulating-supply", circulating_supply_routes())
                    .nest("/status", node_status_routes(network_monitor))
                    .nest("/network", nym_network_routes())
                    .nest("/api-status", status::handlers::api_status_routes())
                    .nest("/nym-nodes", nym_node_routes())
                    .nest("/unstable", unstable_routes()), // CORS layer needs to be "outside" of routes
            );

        Self {
            unfinished_router: default_routes,
        }
    }

    pub(crate) fn nest(self, path: &str, router: Router<AppState>) -> Self {
        Self {
            unfinished_router: self.unfinished_router.nest(path, router),
        }
    }

    /// Invoke this as late as possible before constructing HTTP server
    /// (after all routes were added).
    pub(crate) fn with_state(self, state: AppState) -> RouterWithState {
        RouterWithState {
            router: self.finalize_routes().with_state(state),
        }
    }

    /// Middleware added here intercepts the request before it gets to other routes.
    fn finalize_routes(self) -> Router<AppState> {
        self.unfinished_router
            .layer(setup_cors())
            .layer(axum::middleware::from_fn(logger))
    }
}

fn setup_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers(tower_http::cors::Any)
        .allow_credentials(false)
}

pub(crate) struct RouterWithState {
    router: Router,
}

impl RouterWithState {
    pub(crate) async fn build_server(
        self,
        bind_address: &SocketAddr,
    ) -> anyhow::Result<ApiHttpServer> {
        let listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .map_err(|err| anyhow!("Couldn't bind to address {} due to {}", bind_address, err))?;

        Ok(ApiHttpServer {
            router: self.router,
            listener,
        })
    }
}

pub(crate) struct ApiHttpServer {
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
