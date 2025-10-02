// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use crate::error::ZkNymError;
use crate::vpn_api_client::types::{
    AttributesResponse, MasterVerificationKeyResponse, PartialVerificationKeysResponse,
};
use async_trait::async_trait;
pub use nym_http_api_client::Client;
use nym_http_api_client::{
    parse_response, ApiClient, HttpClientError, IntoUrl, PathSegments, NO_PARAMS,
};
use serde::de::DeserializeOwned;

#[allow(dead_code)]
pub struct VpnApiClient {
    inner: Client,
    bearer_token: String,
}

#[allow(dead_code)]
pub fn new_client(
    base_url: impl IntoUrl,
    bearer_token: impl Into<String>,
) -> Result<VpnApiClient, ZkNymError> {
    Ok(VpnApiClient {
        inner: Client::builder(base_url)
            .map_err(Box::new)?
            .with_user_agent(format!("nym-wasm-znym-lib/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Box::new)?,
        bearer_token: bearer_token.into(),
    })
}

// TODO: do it properly by implementing auth headers on `ApiClient` trait
#[allow(dead_code)]
#[async_trait(?Send)]
pub trait NymVpnApiClient {
    async fn simple_get<T>(&self, path: PathSegments<'_>) -> Result<T, HttpClientError>
    where
        T: DeserializeOwned;

    async fn get_prehashed_public_attributes(&self) -> Result<AttributesResponse, HttpClientError> {
        self.simple_get(&[
            "/api",
            "/v1",
            "/bandwidth-voucher",
            "/prehashed-public-attributes",
        ])
        .await
    }

    async fn get_partial_verification_keys(
        &self,
    ) -> Result<PartialVerificationKeysResponse, HttpClientError> {
        self.simple_get(&[
            "/api",
            "/v1",
            "/bandwidth-voucher",
            "/partial-verification-keys",
        ])
        .await
    }

    async fn get_master_verification_key(
        &self,
    ) -> Result<MasterVerificationKeyResponse, HttpClientError> {
        self.simple_get(&[
            "/api",
            "/v1",
            "/bandwidth-voucher",
            "/master-verification-key",
        ])
        .await
    }
}

#[async_trait(?Send)]
impl NymVpnApiClient for VpnApiClient {
    async fn simple_get<T>(&self, path: PathSegments<'_>) -> Result<T, HttpClientError>
    where
        T: DeserializeOwned,
    {
        let req = self
            .inner
            .create_get_request(path, NO_PARAMS)?
            .bearer_auth(&self.bearer_token)
            .send();

        // the only reason for that target lock is so that I could call this method from an ephemeral test
        // running in non-wasm mode (since I wanted to use tokio)

        #[cfg(target_arch = "wasm32")]
        let res = wasmtimer::tokio::timeout(std::time::Duration::from_secs(5), req)
            .await
            .map_err(|_timeout| HttpClientError::RequestTimeout)??;

        #[cfg(not(target_arch = "wasm32"))]
        let res = req.await?;

        parse_response(res, false).await
    }
}
