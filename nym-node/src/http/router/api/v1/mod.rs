// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::api::v1::roles::NodeRoles;
use crate::http::router::api::v1::build_info::build_info;
use crate::http::router::api::v1::roles::roles;
use crate::http::state::AppState;
use axum::routing::get;
use axum::Router;
use nym_bin_common::build_information::BinaryBuildInformationOwned;

pub mod build_info;
pub mod gateway;
pub mod mixnode;
pub mod network_requester;
pub mod openapi;
pub mod roles;

pub(crate) mod routes {
    pub(crate) const GATEWAY: &str = "/gateway";
    pub(crate) const MIXNODE: &str = "/mixnode";
    pub(crate) const NETWORK_REQUESTER: &str = "/network-requester";
    pub(crate) const BUILD_INFO: &str = "/build-info";
    pub(crate) const ROLES: &str = "/roles";
    pub(crate) const SWAGGER: &str = "/swagger";
}

#[derive(Debug, Clone)]
pub struct Config {
    pub build_information: BinaryBuildInformationOwned,
    pub roles: NodeRoles,
    pub gateway: gateway::Config,
    pub mixnode: mixnode::Config,
    pub network_requester: network_requester::Config,
}

pub(super) fn routes(config: Config) -> Router<AppState> {
    Router::new()
        .nest(routes::GATEWAY, gateway::routes(config.gateway))
        .nest(routes::MIXNODE, mixnode::routes(config.mixnode))
        .nest(
            routes::NETWORK_REQUESTER,
            network_requester::routes(config.network_requester),
        )
        .route(
            routes::BUILD_INFO,
            get({
                let build_information = config.build_information;
                move |query| build_info(build_information, query)
            }),
        )
        .route(
            routes::ROLES,
            get({
                let node_roles = config.roles;
                move |query| roles(node_roles, query)
            }),
        )
        .merge(openapi::route())
}
