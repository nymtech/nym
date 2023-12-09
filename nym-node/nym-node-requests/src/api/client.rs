// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api::v1::gateway::models::WebSockets;
use crate::api::v1::node::models::SignedHostInformation;
use crate::api::ErrorResponse;
use crate::routes;
use async_trait::async_trait;
use http_api_client::{ApiClient, HttpClientError};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_wireguard_types::{ClientMessage, ClientRegistrationResponse};

use crate::api::v1::health::models::NodeHealth;
use crate::api::v1::ip_packet_router::models::IpPacketRouter;
use crate::api::v1::network_requester::exit_policy::models::UsedExitPolicy;
use crate::api::v1::network_requester::models::NetworkRequester;
pub use http_api_client::Client;

pub type NymNodeApiClientError = HttpClientError<ErrorResponse>;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NymNodeApiClientExt: ApiClient {
    async fn get_health(&self) -> Result<NodeHealth, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::health_absolute()).await
    }

    async fn get_host_information(&self) -> Result<SignedHostInformation, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::host_info_absolute())
            .await
    }

    async fn get_build_information(
        &self,
    ) -> Result<BinaryBuildInformationOwned, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::build_info_absolute())
            .await
    }

    // TODO: implement calls for other endpoints; for now I only care about the wss
    async fn get_mixnet_websockets(&self) -> Result<WebSockets, NymNodeApiClientError> {
        self.get_json_from(
            routes::api::v1::gateway::client_interfaces::mixnet_websockets_absolute(),
        )
        .await
    }

    async fn get_network_requester(&self) -> Result<NetworkRequester, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::network_requester_absolute())
            .await
    }

    async fn get_exit_policy(&self) -> Result<UsedExitPolicy, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::network_requester::exit_policy_absolute())
            .await
    }

    async fn get_ip_packet_router(&self) -> Result<IpPacketRouter, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::ip_packet_router_absolute())
            .await
    }

    async fn post_gateway_register_client(
        &self,
        client_message: &ClientMessage,
    ) -> Result<ClientRegistrationResponse, NymNodeApiClientError> {
        self.post_json_data_to(
            routes::api::v1::gateway::client_interfaces::wireguard::client_absolute(),
            client_message,
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl NymNodeApiClientExt for Client {}
