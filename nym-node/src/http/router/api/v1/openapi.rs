// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api;
use crate::http::state::AppState;
use axum::Router;
use nym_node_requests::api as api_requests;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(title = "NymNode API"),
    paths(
        api::v1::node::build_information::build_information,
        api::v1::node::host_information::host_information,
        api::v1::node::roles::roles,
        api::v1::gateway::root::root_gateway,
        api::v1::gateway::client_interfaces::client_interfaces,
        api::v1::gateway::client_interfaces::wireguard,
        api::v1::gateway::client_interfaces::mixnet_websockets,
        api::v1::mixnode::root::root_mixnode,
        api::v1::network_requester::root::root_network_requester,
    ),
    components(schemas(
        api::Output,
        api::OutputParams,
        api_requests::v1::node::models::BinaryBuildInformationOwned,
        api_requests::v1::node::models::SignedHostInformation,
        api_requests::v1::node::models::HostInformation,
        api_requests::v1::node::models::HostKeys,
        api_requests::v1::node::models::NodeRoles,
        api_requests::v1::gateway::models::Gateway,
        api_requests::v1::gateway::models::Wireguard,
        api_requests::v1::gateway::models::ClientInterfaces,
        api_requests::v1::gateway::models::WebSockets,
        api_requests::v1::mixnode::models::Mixnode,
        api_requests::v1::network_requester::models::NetworkRequester,
    ))
)]
pub(crate) struct ApiDoc;

pub(crate) fn route() -> Router<AppState> {
    // provide absolute path to the openapi.json
    let config = utoipa_swagger_ui::Config::from("/api/v1/api-docs/openapi.json");
    SwaggerUi::new(super::routes::SWAGGER)
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .config(config)
        .into()
}
