// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::api::v1::node::build_information::build_information;
use crate::http::api::v1::node::host_information::host_information;
use crate::http::api::v1::node::roles::roles;
use crate::http::api::v1::node::types::{HostInformation, NodeRoles};
use crate::http::state::AppState;
use axum::routing::get;
use axum::Router;
use nym_bin_common::build_information::BinaryBuildInformationOwned;

pub mod build_information;
pub mod host_information;
pub mod roles;
pub mod types;

pub(crate) mod routes {
    pub(crate) const ROLES: &str = "/roles";
    pub(crate) const BUILD_INFO: &str = "/build-information";
    pub(crate) const HOST_INFO: &str = "/host-information";
}

#[derive(Debug, Clone)]
pub struct Config {
    pub build_information: BinaryBuildInformationOwned,
    pub host_information: HostInformation,
    pub roles: NodeRoles,
}

pub(super) fn routes(config: Config) -> Router<AppState> {
    Router::new()
        .route(
            routes::BUILD_INFO,
            get({
                let build_info = config.build_information;
                move |query| build_information(build_info, query)
            }),
        )
        .route(
            routes::ROLES,
            get({
                let node_roles = config.roles;
                move |query| roles(node_roles, query)
            }),
        )
        .route(
            routes::HOST_INFO,
            get({
                let host_info = config.host_information;
                move |query| host_information(host_info, query)
            }),
        )
}
