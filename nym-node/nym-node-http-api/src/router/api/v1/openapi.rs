// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::router::api;
use crate::router::types::{ErrorResponse, RequestError};
use axum::Router;
use nym_node_requests::api as api_requests;
use nym_node_requests::routes::api::{v1, v1_absolute};
use utoipa::openapi::security::{Http, HttpAuthScheme};
use utoipa::{openapi::security::SecurityScheme, Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(title = "NymNode API"),
    paths(
        api::v1::node::build_information::build_information,
        api::v1::node::host_information::host_information,
        api::v1::node::roles::roles,
        api::v1::node::hardware::host_system,
        api::v1::node::description::description,
        api::v1::metrics::mixing::mixing_stats,
        api::v1::metrics::verloc::verloc_stats,
        api::v1::metrics::prometheus::prometheus_metrics,
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
            api_requests::v1::node::models::HostSystem,
            api_requests::v1::node::models::Hardware,
            api_requests::v1::node::models::Cpu,
            api_requests::v1::node::models::CryptoHardware,
            api_requests::v1::node::models::NodeDescription,
            api_requests::v1::metrics::models::MixingStats,
            api_requests::v1::metrics::models::VerlocStats,
            api_requests::v1::metrics::models::VerlocResult,
            api_requests::v1::metrics::models::VerlocResultData,
            api_requests::v1::metrics::models::VerlocNodeResult,
            api_requests::v1::metrics::models::VerlocMeasurement,
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
        responses(RequestError),
    ),
    modifiers(&SecurityAddon),
)]
pub(crate) struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "prometheus_token",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            )
        }
    }
}

pub(crate) fn route<S: Send + Sync + 'static + Clone>() -> Router<S> {
    // provide absolute path to the openapi.json
    let config =
        utoipa_swagger_ui::Config::from(format!("{}/api-docs/openapi.json", v1_absolute()));
    SwaggerUi::new(v1::SWAGGER)
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .config(config)
        .into()
}
