// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::Router;
use nym_network_monitor_orchestrator_requests::routes;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;
use utoipauto::utoipauto;

// manually import external structs which are behind feature flags because they
// can't be automatically discovered
// https://github.com/ProbablyClem/utoipauto/issues/13#issuecomment-1974911829
#[utoipauto(
    paths = "./nym-network-monitor-v3/nym-network-monitor-orchestrator/src",
    "./nym-network-monitor-v3/nym-network-monitor-orchestrator-requests/src from nym-network-monitor-orchestrator-requests"
)]
#[derive(OpenApi)]
#[openapi(
    info(title = "Nym Network Monitor Orchestrator API"),
    tags(),
    modifiers(&SecurityAddon),
)]
pub(crate) struct ApiDoc;

/// OpenAPI modifier that registers bearer-token security schemes for the API docs.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // token authorising access to prometheus metrics and test-run results
            components.add_security_scheme(
                "metrics_and_results_token",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );

            // token authorising monitor agents
            components.add_security_scheme(
                "agents_token",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}

/// Returns a router that serves the Swagger UI and the generated OpenAPI JSON spec.
pub(crate) fn route<S: Send + Sync + 'static + Clone>() -> Router<S> {
    SwaggerUi::new(routes::SWAGGER)
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .into()
}
