use anyhow::anyhow;
use axum::Router;
use tokio::net::ToSocketAddrs;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::http::{server::HttpServer, state::AppState};

// TODO dz src/http/gateways.rs
mod gateways;
// TODO dz src/http/mixnodes.rs
mod mixnodes;
// TODO dz src/http/services.rs
mod services;
// TODO dz src/http/summary.rs
mod summary;
// TODO dz src/http/testruns.rs
mod testruns;

pub(crate) struct RouterBuilder {
    unfinished_router: Router<AppState>,
}

impl RouterBuilder {
    pub(crate) fn with_default_routes() -> Self {
        let router = Router::new()
            .merge(
                SwaggerUi::new("/")
                    .url("/api-docs/openapi.json", super::api_docs::ApiDoc::openapi()),
            )
            .nest(
                "/v2",
                Router::new()
                    .merge(gateways::routes())
                    .merge(mixnodes::routes())
                    .merge(services::routes())
                    .merge(summary::routes())
                    .merge(testruns::routes()),
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
            .layer(TraceLayer::new_for_http())
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
        .allow_credentials(true)
}
