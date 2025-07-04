// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_http_api_client::{ApiClient, NO_PARAMS};

use url::Url;

use crate::{
    client::NYM_VPN_API_TIMEOUT,
    error::{Result, VpnApiClientError},
    response::{NymWellknownDiscoveryItemResponse, RegisteredNetworksResponse},
    routes,
};

/// Bootstrapping Environments and Network Discovery
pub struct BootstrapVpnApiClient {
    inner: nym_http_api_client::Client,
}

impl BootstrapVpnApiClient {
    /// Returns a VpnApiClient Based on locally set well known url and empty user agent.
    ///
    /// THIS SHOULD ONLY BE USED FOR BOOTSTRAPPING.
    pub fn new(base_url: Url) -> Result<Self> {
        nym_http_api_client::Client::builder(base_url)
            .map(|builder| builder.with_timeout(NYM_VPN_API_TIMEOUT))
            .and_then(|builder| builder.build())
            .map(|c| Self { inner: c })
            .map_err(VpnApiClientError::CreateVpnApiClient)
    }

    pub async fn get_wellknown_envs(&self) -> Result<RegisteredNetworksResponse> {
        self.inner
            .get_json(
                &[
                    routes::PUBLIC,
                    routes::V1,
                    routes::WELLKNOWN,
                    routes::ENVS_FILE,
                ],
                NO_PARAMS,
            )
            .await
            .map_err(VpnApiClientError::GetNetworkEnvs)
    }

    pub async fn get_wellknown_discovery(
        &self,
        network_name: &str,
    ) -> Result<NymWellknownDiscoveryItemResponse> {
        self.inner
            .get_json(
                &[
                    routes::PUBLIC,
                    routes::V1,
                    routes::WELLKNOWN,
                    network_name,
                    routes::DISCOVERY_FILE,
                ],
                NO_PARAMS,
            )
            .await
            .map_err(VpnApiClientError::GetDiscoveryInfo)
    }
}
