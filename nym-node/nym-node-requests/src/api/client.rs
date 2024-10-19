// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api::v1::gateway::models::WebSockets;
use crate::api::v1::node::models::{
    AuxiliaryDetails, NodeDescription, NodeRoles, SignedHostInformation,
};
use crate::api::ErrorResponse;
use crate::routes;
use async_trait::async_trait;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_http_api_client::{ApiClient, HttpClientError};

use crate::api::v1::authenticator::models::Authenticator;
use crate::api::v1::health::models::NodeHealth;
use crate::api::v1::ip_packet_router::models::IpPacketRouter;
use crate::api::v1::network_requester::exit_policy::models::UsedExitPolicy;
use crate::api::v1::network_requester::models::NetworkRequester;
pub use nym_http_api_client::Client;

use super::v1::gateway::models::Wireguard;

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

    async fn get_description(&self) -> Result<NodeDescription, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::description_absolute())
            .await
    }

    async fn get_build_information(
        &self,
    ) -> Result<BinaryBuildInformationOwned, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::build_info_absolute())
            .await
    }

    async fn get_roles(&self) -> Result<NodeRoles, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::roles_absolute()).await
    }

    async fn get_auxiliary_details(&self) -> Result<AuxiliaryDetails, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::auxiliary_absolute())
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

    async fn get_authenticator(&self) -> Result<Authenticator, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::authenticator_absolute())
            .await
    }

    async fn get_wireguard(&self) -> Result<Wireguard, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::gateway::client_interfaces::wireguard_absolute())
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl NymNodeApiClientExt for Client {}
