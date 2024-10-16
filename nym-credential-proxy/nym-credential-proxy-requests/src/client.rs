// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::ticketbook::models::{
    MasterVerificationKeyResponse, PartialVerificationKeysResponse, TicketbookRequest,
    TicketbookWalletSharesResponse,
};
use async_trait::async_trait;
use nym_http_api_client::{parse_response, HttpClientError, Params, PathSegments, NO_PARAMS};
use reqwest::IntoUrl;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub use nym_http_api_client::Client;
pub type VpnApiClientError = HttpClientError;

#[allow(dead_code)]
pub struct VpnApiClient {
    inner: Client,
    bearer_token: String,
}

pub fn new_client(
    base_url: impl IntoUrl,
    bearer_token: impl Into<String>,
) -> Result<VpnApiClient, VpnApiClientError> {
    Ok(VpnApiClient {
        inner: Client::builder(base_url)?
            .with_user_agent(format!(
                "nym-credential-proxy-requests/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?,
        bearer_token: bearer_token.into(),
    })
}

// TODO: do it properly by implementing auth headers on `ApiClient` trait
#[allow(dead_code)]
#[async_trait(?Send)]
pub trait NymVpnApiClient {
    async fn simple_get<T>(&self, path: PathSegments<'_>) -> Result<T, VpnApiClientError>
    where
        T: DeserializeOwned;

    async fn simple_post<B, T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<T, VpnApiClientError>
    where
        B: Serialize + ?Sized,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str>,
        V: AsRef<str>;

    async fn get_partial_verification_keys(
        &self,
    ) -> Result<PartialVerificationKeysResponse, VpnApiClientError> {
        self.simple_get(&["/api", "/v1", "/ticketbook", "/partial-verification-keys"])
            .await
    }

    async fn get_master_verification_key(
        &self,
    ) -> Result<MasterVerificationKeyResponse, VpnApiClientError> {
        self.simple_get(&["/api", "/v1", "/ticketbook", "/master-verification-key"])
            .await
    }

    async fn get_ticketbook_wallet_shares(
        &self,
        request: &TicketbookRequest,
        full_response: bool,
    ) -> Result<TicketbookWalletSharesResponse, VpnApiClientError> {
        let params = vec![("full-response", full_response.to_string())];

        self.simple_post(&["/api", "/v1", "/ticketbook", "/obtain"], &params, request)
            .await
    }
    //
    // async fn get_bandwidth_voucher_blinded_shares(
    //     &self,
    //     blind_sign_request: BlindSignRequest,
    // ) -> Result<BandwidthVoucherResponse, VpnApiClientError>;
}

#[async_trait(?Send)]
impl NymVpnApiClient for VpnApiClient {
    async fn simple_get<T>(&self, path: PathSegments<'_>) -> Result<T, VpnApiClientError>
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

    async fn simple_post<B, T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<T, VpnApiClientError>
    where
        B: Serialize + ?Sized,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let req = self
            .inner
            .create_post_request(path, params, json_body)
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

    // async fn get_bandwidth_voucher_blinded_shares(
    //     &self,
    //     blind_sign_request: BlindSignRequest,
    // ) -> Result<BandwidthVoucherResponse, VpnApiClientError> {
    //     let req = self.inner.create_post_request(
    //         &["/api", "/v1", "/bandwidth-voucher", "/obtain"],
    //         NO_PARAMS,
    //         &BandwidthVoucherRequest { blind_sign_request },
    //     );
    //
    //     let fut = req.bearer_auth(&self.bearer_token).send();
    //
    //     // the only reason for that target lock is so that I could call this method from an ephemeral test
    //     // running in non-wasm mode (since I wanted to use tokio)
    //
    //     #[cfg(target_arch = "wasm32")]
    //     let res = wasmtimer::tokio::timeout(std::time::Duration::from_secs(5), fut)
    //         .await
    //         .map_err(|_timeout| HttpClientError::RequestTimeout)??;
    //
    //     #[cfg(not(target_arch = "wasm32"))]
    //     let res = fut.await?;
    //
    //     parse_response(res, false).await
    // }
}
