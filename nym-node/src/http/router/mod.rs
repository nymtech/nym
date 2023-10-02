// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymNodeError;
use crate::http::api::v1::gateway::types::Gateway;
use crate::http::api::v1::mixnode::types::Mixnode;
use crate::http::api::v1::network_requester::types::NetworkRequester;
use crate::http::api::v1::node::types::SignedHostInformation;
use crate::http::middleware::logging;
use crate::http::state::AppState;
use crate::http::NymNodeHTTPServer;
use axum::Router;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use std::net::SocketAddr;

pub mod api;
pub mod landing_page;
pub mod policy;

pub(crate) mod routes {
    pub(crate) const LANDING_PAGE: &str = "/";
    pub(crate) const POLICY: &str = "/policy";

    pub(crate) const API: &str = "/api";
}

#[derive(Debug, Clone)]
pub struct Config {
    pub landing: landing_page::Config,
    pub policy: policy::Config,
    pub api: api::Config,
}

impl Config {
    pub fn new(
        build_information: BinaryBuildInformationOwned,
        host_information: SignedHostInformation,
    ) -> Self {
        Config {
            landing: Default::default(),
            policy: Default::default(),
            api: api::Config {
                v1_config: api::v1::Config {
                    node: api::v1::node::Config {
                        build_information,
                        host_information,
                        roles: Default::default(),
                    },
                    gateway: Default::default(),
                    mixnode: Default::default(),
                    network_requester: Default::default(),
                },
            },
        }
    }

    #[must_use]
    pub fn with_gateway(mut self, gateway: Gateway) -> Self {
        self.api.v1_config.node.roles.gateway_enabled = true;
        self.api.v1_config.gateway.details = Some(gateway);
        self
    }

    #[must_use]
    pub fn with_mixnode(mut self, mixnode: Mixnode) -> Self {
        self.api.v1_config.node.roles.mixnode_enabled = true;
        self.api.v1_config.mixnode.details = Some(mixnode);
        self
    }

    #[must_use]
    pub fn with_network_requester(mut self, network_requester: NetworkRequester) -> Self {
        self.api.v1_config.node.roles.network_requester_enabled = true;
        self.api.v1_config.network_requester.details = Some(network_requester);
        self
    }
}

pub struct NymNodeRouter {
    inner: Router,
}

impl NymNodeRouter {
    pub fn new(config: Config) -> NymNodeRouter {
        let state = AppState::new();

        NymNodeRouter {
            inner: Router::new()
                .nest(routes::LANDING_PAGE, landing_page::routes(config.landing))
                .nest(routes::POLICY, policy::routes(config.policy))
                .nest(routes::API, api::routes(config.api))
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

    pub fn build_server(
        self,
        bind_address: &SocketAddr,
    ) -> Result<NymNodeHTTPServer, NymNodeError> {
        let axum_server = axum::Server::try_bind(bind_address)
            .map_err(|source| NymNodeError::HttpBindFailure {
                bind_address: *bind_address,
                source,
            })?
            .serve(self.inner.into_make_service());

        Ok(NymNodeHTTPServer::new(axum_server))
    }
}
