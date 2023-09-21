// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymNodeError;
use crate::http::middleware::logging;
use axum::routing::IntoMakeService;
use axum::Router;
use hyper::server::conn::AddrIncoming;
use hyper::Server;
use std::net::SocketAddr;

pub mod api;
pub mod landing_page;
mod policy;

pub(crate) mod routes {
    pub(crate) const LANDING_PAGE: &str = "/";
    pub(crate) const POLICY: &str = "/policy";

    pub(crate) const API: &str = "/api";
}

// TODO: can it be made nicer?
pub type NymNodeHTTPServer = Server<AddrIncoming, IntoMakeService<Router>>;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub landing: landing_page::Config,
    pub policy: policy::Config,
    pub api: api::Config,
}

pub struct NymNodeRouter {
    // landing_page_assets: Option<PathBuf>,
    inner: Router,
}

impl NymNodeRouter {
    pub fn new(config: Config) -> NymNodeRouter {
        /*
        .merge(SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi()));
        */
        use utoipa::OpenApi;
        NymNodeRouter {
            inner: Router::new()
                .nest(routes::LANDING_PAGE, landing_page::routes(config.landing))
                .nest(routes::POLICY, policy::routes(config.policy))
                .nest(routes::API, api::routes(config.api))
                .layer(axum::middleware::from_fn(logging::logger)),
        }
    }

    // this is only a temporary method until everything is properly moved into the nym-node itself
    #[must_use]
    pub fn with_route(mut self, path: &str, router: Router) -> Self {
        self.inner = self.inner.nest(path, router);
        self
    }

    pub fn build_server(
        self,
        bind_address: &SocketAddr,
    ) -> Result<NymNodeHTTPServer, NymNodeError> {
        Ok(axum::Server::try_bind(bind_address)
            .map_err(|source| NymNodeError::HttpBindFailure {
                bind_address: *bind_address,
                source,
            })?
            .serve(self.inner.into_make_service()))
    }
}
