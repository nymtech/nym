// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api;
use crate::http::state::AppState;
use axum::Router;
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
        api::v1::node::types::BinaryBuildInformationOwned,
        api::v1::node::types::HostInformation,
        api::v1::node::types::HostKeys,
        api::v1::node::types::NodeRoles,
        api::v1::gateway::types::Gateway,
        api::v1::gateway::types::Wireguard,
        api::v1::gateway::types::ClientInterfaces,
        api::v1::gateway::types::WebSockets,
        api::v1::mixnode::types::Mixnode,
        api::v1::network_requester::types::NetworkRequester,
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
