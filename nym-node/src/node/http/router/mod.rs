// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::NymNodeHttpServer;
use crate::node::http::error::NymNodeHttpError;
use crate::node::http::state::AppState;
use axum::Router;
use axum::response::Redirect;
use axum::routing::get;
use nym_http_api_common::middleware::logging;
use nym_node_requests::api::v1::authenticator::models::Authenticator;
use nym_node_requests::api::v1::gateway::models::{Bridges, Gateway};
use nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter;
use nym_node_requests::api::v1::mixnode::models::Mixnode;
use nym_node_requests::api::v1::network_requester::exit_policy::models::UsedExitPolicy;
use nym_node_requests::api::v1::network_requester::models::NetworkRequester;
use nym_node_requests::routes;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio_util::sync::WaitForCancellationFutureOwned;
use zeroize::Zeroizing;

pub mod api;
pub mod landing_page;
pub mod types;

#[derive(Debug, Clone)]
pub struct HttpServerConfig {
    pub landing: landing_page::Config,
    pub api: api::Config,
}

impl HttpServerConfig {
    pub fn new() -> Self {
        HttpServerConfig {
            landing: Default::default(),
            api: api::Config {
                v1_config: api::v1::Config {
                    node: api::v1::node::Config {},
                    metrics: Default::default(),
                    gateway: Default::default(),
                    mixnode: Default::default(),
                    bridges: Default::default(),
                    network_requester: Default::default(),
                    ip_packet_router: Default::default(),
                    authenticator: Default::default(),
                    lewes_protocol: Default::default(),
                },
                v2_config: api::v2::Config {
                    node: api::v2::node::Config {},
                },
            },
        }
    }

    #[must_use]
    pub fn with_landing_page_assets<P: AsRef<Path>>(mut self, assets_path: Option<P>) -> Self {
        self.landing.assets_path = assets_path.map(|p| p.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn with_gateway_details(mut self, gateway: Gateway) -> Self {
        self.api.v1_config.gateway.details = Some(gateway);
        self
    }

    #[must_use]
    pub fn with_mixnode_details(mut self, mixnode: Mixnode) -> Self {
        self.api.v1_config.mixnode.details = Some(mixnode);
        self
    }

    #[must_use]
    pub fn with_network_requester_details(mut self, network_requester: NetworkRequester) -> Self {
        self.api.v1_config.network_requester.details = Some(network_requester);
        self
    }

    #[must_use]
    pub fn with_used_exit_policy(mut self, exit_policy: UsedExitPolicy) -> Self {
        self.api.v1_config.network_requester.exit_policy = Some(exit_policy);
        self
    }

    #[must_use]
    pub fn with_ip_packet_router_details(mut self, ip_packet_router: IpPacketRouter) -> Self {
        self.api.v1_config.ip_packet_router.details = Some(ip_packet_router);
        self
    }

    #[must_use]
    pub fn with_authenticator_details(mut self, authenticator: Authenticator) -> Self {
        self.api.v1_config.authenticator.details = Some(authenticator);
        self
    }

    #[must_use]
    pub fn with_prometheus_bearer_token(mut self, bearer_token: Option<String>) -> Self {
        self.api.v1_config.metrics.bearer_token = bearer_token.map(|b| Arc::new(Zeroizing::new(b)));
        self
    }

    #[must_use]
    pub fn with_bridge_client_params_file(mut self, path: &Path) -> Self {
        self.api.v1_config.bridges.details = Some(Bridges {
            client_params_path: path.to_string_lossy().to_string(),
        });
        self
    }

    // #[must_use]
    // pub fn with_lewes_protocol(mut self, lewes_protocol: LewesProtocol) -> Self {
    //     self.api.v1_config.lewes_protocol.details = Some(lewes_protocol);
    //     self
    // }
}

pub struct NymNodeRouter {
    inner: Router,
}

impl NymNodeRouter {
    pub fn new(config: HttpServerConfig, state: AppState) -> NymNodeRouter {
        NymNodeRouter {
            inner: Router::new()
                // redirection for old legacy mixnode routes
                .route(
                    "/hardware",
                    get(|| async { Redirect::to(&routes::api::v1::system_info_absolute()) }),
                )
                .route(
                    "/description",
                    get(|| async { Redirect::to(&routes::api::v1::description_absolute()) }),
                )
                .route(
                    "/stats",
                    get(|| async {
                        Redirect::to(&routes::api::v1::metrics::legacy_mixing_absolute())
                    }),
                )
                .route(
                    "/verloc",
                    get(|| async { Redirect::to(&routes::api::v1::metrics::verloc_absolute()) }),
                )
                .route(
                    "/metrics",
                    get(|| async {
                        Redirect::to(&routes::api::v1::metrics::prometheus_absolute())
                    }),
                )
                .merge(landing_page::routes(config.landing))
                .nest(routes::API, api::routes(config.api))
                // openapi must be merged at the outer router level (not nested) —
                // SwaggerUi emits internal redirects that use absolute paths
                // unaware of any `.nest()` prefix
                .merge(api::openapi::route())
                .layer(axum::middleware::from_fn(logging::log_request_info))
                .with_state(state),
        }
    }

    pub async fn build_server(
        self,
        bind_address: &SocketAddr,
        shutdown: WaitForCancellationFutureOwned,
    ) -> Result<NymNodeHttpServer, NymNodeHttpError> {
        let listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .map_err(|source| NymNodeHttpError::HttpBindFailure {
                bind_address: *bind_address,
                source,
            })?;

        Ok(axum::serve(
            listener,
            self.inner
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_constructs_without_panic() {
        let config = HttpServerConfig::new();
        let _ = NymNodeRouter::new(config, AppState::dummy());
    }
}
