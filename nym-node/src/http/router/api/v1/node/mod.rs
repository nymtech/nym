// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::node::build_information::build_information;
use crate::http::api::v1::node::host_information::host_information;
use crate::http::api::v1::node::noise_information::noise_information;
use crate::http::api::v1::node::roles::roles;
use axum::routing::get;
use axum::Router;
use nym_node_requests::api::v1::node::models;
use nym_node_requests::routes::api::v1;

pub mod build_information;
pub mod host_information;
pub mod noise_information;
pub mod roles;

#[derive(Debug, Clone)]
pub struct Config {
    pub build_information: models::BinaryBuildInformationOwned,
    pub host_information: models::SignedHostInformation,
    pub noise_information: models::NoiseInformation,
    pub roles: models::NodeRoles,
}

pub(super) fn routes<S: Send + Sync + 'static + Clone>(config: Config) -> Router<S> {
    Router::new()
        .route(
            v1::BUILD_INFO,
            get({
                let build_info = config.build_information;
                move |query| build_information(build_info, query)
            }),
        )
        .route(
            v1::ROLES,
            get({
                let node_roles = config.roles;
                move |query| roles(node_roles, query)
            }),
        )
        .route(
            v1::HOST_INFO,
            get({
                let host_info = config.host_information;
                move |query| host_information(host_info, query)
            }),
        )
        .route(
            v1::NOISE_INFO,
            get({
                let noise_info = config.noise_information;
                move |query| noise_information(noise_info, query)
            }),
        )
}
