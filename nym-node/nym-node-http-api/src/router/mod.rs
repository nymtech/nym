// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub use crate::api::v1::gateway::client_interfaces::wireguard::WireguardAppState;
use crate::error::NymNodeHttpError;
use crate::middleware::logging;
use crate::state::AppState;
use crate::NymNodeHTTPServer;
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use nym_node_requests::api::v1::gateway::models::{Gateway, Wireguard};
use nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter;
use nym_node_requests::api::v1::mixnode::models::Mixnode;
use nym_node_requests::api::v1::network_requester::exit_policy::models::UsedExitPolicy;
use nym_node_requests::api::v1::network_requester::models::NetworkRequester;
use nym_node_requests::api::v1::node::models;
use nym_node_requests::api::v1::node::models::{HostSystem, NodeDescription};
use nym_node_requests::api::SignedHostInformation;
use nym_node_requests::routes;
use std::net::SocketAddr;
use std::path::Path;
use tracing::warn;

pub mod api;
pub mod landing_page;
pub mod types;

#[derive(Debug, Clone)]
pub struct Config {
    pub landing: landing_page::Config,
    pub api: api::Config,
}

impl Config {
    pub fn new(
        build_information: models::BinaryBuildInformationOwned,
        host_information: SignedHostInformation,
    ) -> Self {
        Config {
            landing: Default::default(),
            api: api::Config {
                v1_config: api::v1::Config {
                    node: api::v1::node::Config {
                        build_information,
                        host_information,
                        system_info: None,
                        roles: Default::default(),
                        description: Default::default(),
                    },
                    metrics: Default::default(),
                    gateway: Default::default(),
                    mixnode: Default::default(),
                    network_requester: Default::default(),
                    ip_packet_router: Default::default(),
                },
            },
        }
    }

    pub fn with_wireguard_interface(mut self, wireguard: Wireguard) -> Self {
        match &mut self.api.v1_config.gateway.details {
            Some(gw) => gw.client_interfaces.wireguard = Some(wireguard),
            None => {
                warn!(
                    "can't add wireguard interface information as the gateway role is not enabled."
                );
            }
        }
        self
    }

    #[must_use]
    pub fn with_landing_page_assets<P: AsRef<Path>>(mut self, assets_path: Option<P>) -> Self {
        self.landing.assets_path = assets_path.map(|p| p.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn with_system_info(mut self, info: HostSystem) -> Self {
        self.api.v1_config.node.system_info = Some(info);
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: NodeDescription) -> Self {
        self.api.v1_config.node.description = description;
        self
    }

    #[must_use]
    pub fn with_gateway(mut self, gateway: Gateway) -> Self {
        self.api.v1_config.node.roles.gateway_enabled = true;
        self.with_gateway_details(gateway)
    }

    #[must_use]
    pub fn with_gateway_details(mut self, gateway: Gateway) -> Self {
        self.api.v1_config.gateway.details = Some(gateway);
        self
    }

    #[must_use]
    pub fn with_mixnode(mut self, mixnode: Mixnode) -> Self {
        self.api.v1_config.node.roles.mixnode_enabled = true;
        self.with_mixnode_details(mixnode)
    }

    #[must_use]
    pub fn with_mixnode_details(mut self, mixnode: Mixnode) -> Self {
        self.api.v1_config.mixnode.details = Some(mixnode);
        self
    }

    #[must_use]
    pub fn with_network_requester(mut self, network_requester: NetworkRequester) -> Self {
        self.api.v1_config.node.roles.network_requester_enabled = true;
        self.with_network_requester_details(network_requester)
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
    pub fn with_ip_packet_router(mut self, ip_packet_router: IpPacketRouter) -> Self {
        self.api.v1_config.node.roles.ip_packet_router_enabled = true;
        self.with_ip_packet_router_details(ip_packet_router)
    }

    #[must_use]
    pub fn with_ip_packet_router_details(mut self, ip_packet_router: IpPacketRouter) -> Self {
        self.api.v1_config.ip_packet_router.details = Some(ip_packet_router);
        self
    }
}

pub struct NymNodeRouter {
    inner: Router,
}

impl NymNodeRouter {
    // TODO: move the wg state to a builder
    pub fn new(
        config: Config,
        app_state: Option<AppState>,
        initial_wg_state: Option<WireguardAppState>,
    ) -> NymNodeRouter {
        let state = app_state.unwrap_or(AppState::new());

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
                    get(|| async { Redirect::to(&routes::api::v1::metrics::mixing_absolute()) }),
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
                .nest(routes::LANDING_PAGE, landing_page::routes(config.landing))
                .nest(
                    routes::API,
                    api::routes(config.api, initial_wg_state.unwrap_or_default()),
                )
                .layer(axum::middleware::from_fn(logging::logger))
                .with_state(state),
        }
    }

    // this is only a temporary method until everything is properly moved into the nym-node itself
    #[must_use]
    pub fn with_route(mut self, path: &str, router: Router) -> Self {
        self.inner = self.inner.nest(path, router);
        self
    }

    #[must_use]
    pub fn with_merged(mut self, router: Router) -> Self {
        self.inner = self.inner.merge(router);
        self
    }

    pub fn build_server(
        self,
        bind_address: &SocketAddr,
    ) -> Result<NymNodeHTTPServer, NymNodeHttpError> {
        let axum_server = axum::Server::try_bind(bind_address)
            .map_err(|source| NymNodeHttpError::HttpBindFailure {
                bind_address: *bind_address,
                source,
            })?
            .serve(
                self.inner
                    .into_make_service_with_connect_info::<SocketAddr>(),
            );

        Ok(NymNodeHTTPServer::new(axum_server))
    }
}
