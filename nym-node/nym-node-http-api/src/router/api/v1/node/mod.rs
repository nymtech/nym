// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::node::auxiliary::auxiliary;
use crate::api::v1::node::build_information::build_information;
use crate::api::v1::node::description::description;
use crate::api::v1::node::hardware::host_system;
use crate::api::v1::node::host_information::host_information;
use crate::api::v1::node::roles::roles;
use axum::routing::get;
use axum::Router;
use nym_node_requests::api::v1::node::models;
use nym_node_requests::routes::api::v1;

pub mod auxiliary;
pub mod build_information;
pub mod description;
pub mod hardware;
pub mod host_information;
pub mod roles;

#[derive(Debug, Clone)]
pub struct Config {
    pub build_information: models::BinaryBuildInformationOwned,
    pub host_information: models::SignedHostInformation,
    pub system_info: Option<models::HostSystem>,
    pub roles: models::NodeRoles,
    pub description: models::NodeDescription,
    pub auxiliary_details: models::AuxiliaryDetails,
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
            v1::SYSTEM_INFO,
            get({
                let system_info = config.system_info;
                move |query| host_system(system_info, query)
            }),
        )
        .route(
            v1::NODE_DESCRIPTION,
            get({
                let node_description = config.description;
                move |query| description(node_description, query)
            }),
        )
        .route(
            v1::AUXILIARY,
            get({
                let auxiliary_details = config.auxiliary_details;
                move |query| auxiliary(auxiliary_details, query)
            }),
        )
}
