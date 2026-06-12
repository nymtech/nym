// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::api::v1::node::auxiliary::auxiliary;
use crate::node::http::api::v1::node::build_information::build_information;
use crate::node::http::api::v1::node::description::description;
use crate::node::http::api::v1::node::hardware::host_system;
use crate::node::http::api::v1::node::host_information::host_information;
use crate::node::http::api::v1::node::roles::roles;
use crate::node::http::state::AppState;
use axum::Router;
use axum::routing::get;
use nym_node_requests::routes::api::v1;

pub mod auxiliary;
pub mod build_information;
pub mod description;
pub mod hardware;
pub mod host_information;
pub mod roles;

#[derive(Debug, Clone, Copy)]
pub struct Config {}

pub(super) fn routes(_config: Config) -> Router<AppState> {
    Router::new()
        .route(v1::BUILD_INFO, get(build_information))
        .route(v1::ROLES, get(roles))
        .route(v1::HOST_INFO, get(host_information))
        .route(v1::SYSTEM_INFO, get(host_system))
        .route(v1::NODE_DESCRIPTION, get(description))
        .route(v1::AUXILIARY, get(auxiliary))
}
