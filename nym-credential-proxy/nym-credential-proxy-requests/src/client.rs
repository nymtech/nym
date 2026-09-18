// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::ticketbook::models::{
    MasterVerificationKeyResponse, PartialVerificationKeysResponse, TicketbookRequest,
    TicketbookWalletSharesResponse,
};
use async_trait::async_trait;
use nym_http_api_client::{
    ApiClient, HttpClientError, NO_PARAMS, Params, PathSegments, parse_response,
};
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

#[allow(clippy::result_large_err)]
pub fn new_client(
    base_url: impl IntoUrl,
    bearer_token: impl Into<String>,
) -> Result<VpnApiClient, VpnApiClientError> {
    let raw = base_url.as_str().to_string();
    let url = base_url
        .into_url()
        .map_err(|source| VpnApiClientError::MalformedUrl { raw, source })?;
    Ok(VpnApiClient {
        inner: Client::builder(url)?
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

    async fn get_with_params<T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<T, VpnApiClientError>
    where
        T: DeserializeOwned,
        K: AsRef<str>,
        V: AsRef<str>;

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

    /// `epoch_id` is the epoch whose signers to ask about. A caller unblinding shares must name
    /// the epoch those shares were issued under: omitting it means whichever epoch is currently
    /// being issued under, and a rotation in between yields keys the shares cannot be verified
    /// against - which renders the ticketbook permanently unusable.
    async fn get_partial_verification_keys(
        &self,
        epoch_id: Option<u64>,
    ) -> Result<PartialVerificationKeysResponse, VpnApiClientError> {
        // note the hyphen: this endpoint's parameters are kebab-case (`EpochIdParams`), unlike
        // nym-api's, which are snake_case. Getting it wrong binds nothing and silently defaults.
        let params = match epoch_id {
            None => Vec::new(),
            Some(epoch_id) => vec![("epoch-id", epoch_id.to_string())],
        };

        self.get_with_params(
            &["/api", "/v1", "/ticketbook", "/partial-verification-keys"],
            &params,
        )
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
        self.get_with_params(path, NO_PARAMS).await
    }

    async fn get_with_params<T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<T, VpnApiClientError>
    where
        T: DeserializeOwned,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let req = self
            .inner
            .create_get_request(path, params)?
            .bearer_auth(&self.bearer_token)
            .build()
            .map_err(VpnApiClientError::reqwest_client_build_error)?;

        let url = req.url().clone();

        let req = reqwest::Client::new().execute(req);

        // the only reason for that target lock is so that I could call this method from an ephemeral test
        // running in non-wasm mode (since I wanted to use tokio)

        #[cfg(target_arch = "wasm32")]
        let res = wasmtimer::tokio::timeout(std::time::Duration::from_secs(5), req)
            .await
            .map_err(|_timeout| HttpClientError::RequestTimeout)??;

        #[cfg(not(target_arch = "wasm32"))]
        let res = req
            .await
            .map_err(|source| VpnApiClientError::request_send_error(url, source))?;
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
            .create_post_request(path, params, json_body)?
            .bearer_auth(&self.bearer_token)
            .build()
            .map_err(VpnApiClientError::reqwest_client_build_error)?;

        let url = req.url().clone();

        let req = reqwest::Client::new().execute(req);

        // the only reason for that target lock is so that I could call this method from an ephemeral test
        // running in non-wasm mode (since I wanted to use tokio)

        #[cfg(target_arch = "wasm32")]
        let res = wasmtimer::tokio::timeout(std::time::Duration::from_secs(5), req)
            .await
            .map_err(|_timeout| HttpClientError::RequestTimeout)??;

        #[cfg(not(target_arch = "wasm32"))]
        let res = req
            .await
            .map_err(|source| VpnApiClientError::request_send_error(url, source))?;

        parse_response(res, false).await
    }
}
