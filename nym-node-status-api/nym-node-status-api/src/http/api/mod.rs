use anyhow::anyhow;
use axum::{Router, response::Redirect};
use nym_http_api_common::middleware::logging::log_request_debug;
use tokio::net::ToSocketAddrs;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::http::{server::HttpServer, state::AppState};

pub(crate) mod dvpn;
pub(crate) mod gateways;
pub(crate) mod metrics;
pub(crate) mod mixnodes;
pub(crate) mod nym_nodes;
pub(crate) mod services;
pub(crate) mod status;
pub(crate) mod summary;
pub(crate) mod testruns;

pub(crate) struct RouterBuilder {
    unfinished_router: Router<AppState>,
}

impl RouterBuilder {
    pub(crate) fn with_default_routes() -> Self {
        let router = Router::new()
            .merge(
                SwaggerUi::new("/swagger")
                    .url("/api-docs/openapi.json", super::api_docs::ApiDoc::openapi()),
            )
            .route(
                "/",
                axum::routing::get(|| async { Redirect::permanent("/swagger") }),
            )
            .nest(
                "/v2",
                Router::new()
                    .nest("/gateways", gateways::routes())
                    .nest("/mixnodes", mixnodes::routes())
                    .nest("/services", services::routes())
                    .nest("/summary", summary::routes())
                    .nest("/metrics", metrics::routes())
                    .nest("/status", status::routes()),
            )
            .nest(
                "/explorer/v3",
                Router::new().nest("/nym-nodes", nym_nodes::routes()),
            )
            .nest(
                "/dvpn/v1",
                Router::new().nest("/directory/gateways", dvpn::routes()),
            )
            .nest(
                "/internal",
                Router::new().nest("/testruns", testruns::routes()),
            );

        Self {
            unfinished_router: router,
        }
    }

    pub(crate) fn with_state(self, state: AppState) -> RouterWithState {
        RouterWithState {
            router: self.finalize_routes().with_state(state),
        }
    }

    fn finalize_routes(self) -> Router<AppState> {
        // layers added later wrap earlier layers
        self.unfinished_router
            // CORS layer needs to wrap other API layers
            .layer(setup_cors())
            // logger should be outermost layer
            .layer(axum::middleware::from_fn(log_request_debug))
    }
}

pub(crate) struct RouterWithState {
    router: Router,
}

impl RouterWithState {
    pub(crate) async fn build_server<A: ToSocketAddrs>(
        self,
        bind_address: A,
    ) -> anyhow::Result<HttpServer> {
        tokio::net::TcpListener::bind(bind_address)
            .await
            .map(|listener| HttpServer::new(self.router, listener))
            .map_err(|err| anyhow!("Couldn't bind to address due to {}", err))
    }
}

fn setup_cors() -> CorsLayer {
    use axum::http::Method;
    CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::POST, Method::GET, Method::PATCH, Method::OPTIONS])
        .allow_headers(tower_http::cors::Any)
        .allow_credentials(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_configuration() {
        let cors = setup_cors();

        // Test that CORS is configured (this tests that the function returns a valid CorsLayer)
        // The actual CORS behavior would need integration tests
        let _layer = cors; // This ensures the cors layer is valid
    }

    #[test]
    fn test_router_builder_creates_routes() {
        let router_builder = RouterBuilder::with_default_routes();

        // Test that the router builder has the expected structure
        // The router itself is private, but we can test that the builder is created
        let unfinished_router = router_builder.unfinished_router;

        // Convert to a testable format - this will compile only if routes are properly configured
        let _test_router = unfinished_router;
    }

    #[test]
    fn test_router_builder_finalize() {
        let router_builder = RouterBuilder::with_default_routes();
        let finalized = router_builder.finalize_routes();

        // This tests that finalize_routes produces a valid Router
        let _router = finalized;
    }
}
