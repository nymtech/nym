// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use tracing::instrument;

use nym_http_api_client::{ApiClient, Client, HttpClientError, NO_PARAMS};

use nym_wireguard_private_metadata_shared::{
    routes, Version, {Request, Response},
};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait WireguardMetadataApiClient: ApiClient {
    #[instrument(level = "debug", skip(self))]
    async fn version(&self) -> Result<Version, HttpClientError> {
        let version: u64 = self
            .get_json(
                &[routes::V1_API_VERSION, routes::BANDWIDTH, routes::VERSION],
                NO_PARAMS,
            )
            .await?;
        Ok(version.into())
    }

    #[instrument(level = "debug", skip(self))]
    async fn available_bandwidth(
        &self,
        request_body: &Request,
    ) -> Result<Response, HttpClientError> {
        self.post_json(
            &[routes::V1_API_VERSION, routes::BANDWIDTH, routes::AVAILABLE],
            NO_PARAMS,
            request_body,
        )
        .await
    }

    #[instrument(level = "debug", skip(self, request_body))]
    async fn topup_bandwidth(&self, request_body: &Request) -> Result<Response, HttpClientError> {
        self.post_json(
            &[routes::V1_API_VERSION, routes::BANDWIDTH, routes::TOPUP],
            NO_PARAMS,
            request_body,
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl WireguardMetadataApiClient for Client {}
