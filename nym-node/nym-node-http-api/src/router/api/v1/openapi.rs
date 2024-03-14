// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::router::api;
use crate::router::types::{ErrorResponse, RequestError};
use axum::Router;
use nym_node_requests::api as api_requests;
use nym_node_requests::routes::api::v1;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(title = "NymNode API"),
    paths(
        api::v1::node::build_information::build_information,
        api::v1::node::host_information::host_information,
        api::v1::node::roles::roles,
        api::v1::health::root_health,
        api::v1::gateway::root::root_gateway,
        api::v1::gateway::client_interfaces::client_interfaces,
        api::v1::gateway::client_interfaces::wireguard_info,
        api::v1::gateway::client_interfaces::mixnet_websockets,
        api::v1::gateway::client_interfaces::wireguard::client_registry::register_client,
        api::v1::gateway::client_interfaces::wireguard::client_registry::get_all_clients,
        api::v1::gateway::client_interfaces::wireguard::client_registry::get_client,
        api::v1::mixnode::root::root_mixnode,
        api::v1::network_requester::root::root_network_requester,
        api::v1::network_requester::exit_policy::node_exit_policy,
        api::v1::ip_packet_router::root::root_ip_packet_router,
    ),
    components(
        schemas(
            ErrorResponse,
            api::Output,
            api::OutputParams,
            api_requests::v1::health::models::NodeHealth,
            api_requests::v1::health::models::NodeStatus,
            api_requests::v1::node::models::BinaryBuildInformationOwned,
            api_requests::v1::node::models::SignedHostInformation,
            api_requests::v1::node::models::HostInformation,
            api_requests::v1::node::models::HostKeys,
            api_requests::v1::node::models::NodeRoles,
            api_requests::v1::gateway::models::Gateway,
            api_requests::v1::gateway::models::Wireguard,
            api_requests::v1::gateway::models::ClientInterfaces,
            api_requests::v1::gateway::models::WebSockets,
            api_requests::v1::gateway::client_interfaces::wireguard::models::ClientMessage,
            api_requests::v1::gateway::client_interfaces::wireguard::models::InitMessage,
            api_requests::v1::gateway::client_interfaces::wireguard::models::GatewayClient,
            api_requests::v1::gateway::client_interfaces::wireguard::models::ClientRegistrationResponse,
            api_requests::v1::mixnode::models::Mixnode,
            api_requests::v1::network_requester::models::NetworkRequester,
            api_requests::v1::network_requester::exit_policy::models::AddressPolicy,
            api_requests::v1::network_requester::exit_policy::models::AddressPolicyRule,
            api_requests::v1::network_requester::exit_policy::models::AddressPolicyAction,
            api_requests::v1::network_requester::exit_policy::models::AddressPortPattern,
            api_requests::v1::network_requester::exit_policy::models::PortRange,
            api_requests::v1::network_requester::exit_policy::models::UsedExitPolicy,
            api_requests::v1::ip_packet_router::models::IpPacketRouter,
        ),
        responses(RequestError)
    )
)]
pub(crate) struct ApiDoc;

pub(crate) fn route<S: Send + Sync + 'static + Clone>() -> Router<S> {
    // provide absolute path to the openapi.json
    let config = utoipa_swagger_ui::Config::from("/api/v1/api-docs/openapi.json");
    SwaggerUi::new(v1::SWAGGER)
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .config(config)
        .into()
}
