// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::NymZkNymFaucetClientError;
use crate::error::ZkNymError;
use crate::zk_nym_faucet_client::types::{
    AttributesResponse, BandwidthVoucherRequest, BandwidthVoucherResponse,
    MasterVerificationKeyResponse, PartialVerificationKeysResponse,
};
use async_trait::async_trait;
use nym_coconut::BlindSignRequest;
pub use nym_http_api_client::Client;
use nym_http_api_client::{parse_response, PathSegments, NO_PARAMS};
use reqwest::IntoUrl;
use serde::de::DeserializeOwned;

#[allow(dead_code)]
pub struct NymZkNymFaucetClientErrorApiClient {
    inner: Client,
    bearer_token: String,
}

#[allow(dead_code)]
pub fn new_client(
    base_url: impl IntoUrl,
    bearer_token: impl Into<String>,
) -> Result<NymZkNymFaucetClientErrorApiClient, ZkNymError> {
    Ok(NymZkNymFaucetClientErrorApiClient {
        inner: Client::builder(base_url)?
            .with_user_agent(format!("nym-wasm-znym-lib/{}", env!("CARGO_PKG_VERSION")))
            .build()?,
        bearer_token: bearer_token.into(),
    })
}

// TODO: do it properly by implementing auth headers on `ApiClient` trait
#[allow(dead_code)]
#[async_trait(?Send)]
pub trait NymNymZkNymFaucetClientErrorApiClient {
    async fn simple_get<T>(&self, path: PathSegments<'_>) -> Result<T, NymZkNymFaucetClientError>
    where
        T: DeserializeOwned;

    async fn get_prehashed_public_attributes(
        &self,
    ) -> Result<AttributesResponse, NymZkNymFaucetClientError> {
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
    ) -> Result<PartialVerificationKeysResponse, NymZkNymFaucetClientError> {
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
    ) -> Result<MasterVerificationKeyResponse, NymZkNymFaucetClientError> {
        self.simple_get(&[
            "/api",
            "/v1",
            "/bandwidth-voucher",
            "/master-verification-key",
        ])
        .await
    }

    async fn get_bandwidth_voucher_blinded_shares(
        &self,
        blind_sign_request: BlindSignRequest,
    ) -> Result<BandwidthVoucherResponse, NymZkNymFaucetClientError>;
}

#[async_trait(?Send)]
impl NymNymZkNymFaucetClientErrorApiClient for NymZkNymFaucetClientErrorApiClient {
    async fn simple_get<T>(&self, path: PathSegments<'_>) -> Result<T, NymZkNymFaucetClientError>
    where
        T: DeserializeOwned,
    {
        let req = self
            .inner
            .create_get_request(path, NO_PARAMS)
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

    async fn get_bandwidth_voucher_blinded_shares(
        &self,
        blind_sign_request: BlindSignRequest,
    ) -> Result<BandwidthVoucherResponse, NymZkNymFaucetClientError> {
        let req = self.inner.create_post_request(
            &["/api", "/v1", "/bandwidth-voucher", "/obtain"],
            NO_PARAMS,
            &BandwidthVoucherRequest { blind_sign_request },
        );

        let fut = req.bearer_auth(&self.bearer_token).send();

        // the only reason for that target lock is so that I could call this method from an ephemeral test
        // running in non-wasm mode (since I wanted to use tokio)

        #[cfg(target_arch = "wasm32")]
        let res = wasmtimer::tokio::timeout(std::time::Duration::from_secs(5), fut)
            .await
            .map_err(|_timeout| HttpClientError::RequestTimeout)??;

        #[cfg(not(target_arch = "wasm32"))]
        let res = fut.await?;

        parse_response(res, false).await
    }
}
