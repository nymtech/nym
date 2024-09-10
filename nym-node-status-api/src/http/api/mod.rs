use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod gateways;
mod mixnodes;
mod services;
mod summary;
mod testruns;

pub(crate) struct RouterBuilder {
    unfinished_router: Router,
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
}

// TODO dz src/http/gateways.rs
// TODO dz src/http/mixnodes.rs
// TODO dz src/http/services.rs
// TODO dz src/http/summary.rs
// TODO dz src/http/testruns.rs
