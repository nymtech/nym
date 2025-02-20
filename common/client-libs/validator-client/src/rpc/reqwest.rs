// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::rpc::TendermintRpcClient;
use async_trait::async_trait;
use base64::Engine;
use cosmrs::tendermint;
use cosmrs::tendermint::{block::Height, evidence::Evidence, Hash};
use nym_http_api_client::{ApiClient, Client, NO_PARAMS};
use reqwest::header::HeaderMap;
use reqwest::{header, RequestBuilder};
use tendermint_rpc::dialect::{v0_34, v0_37, v0_38, LatestDialect};
use tendermint_rpc::{
    client::CompatMode,
    dialect::{self, Dialect},
    endpoint::{self, *},
    query::Query,
    Error, Order, Response, SimpleRequest,
};
use url::Url;

// copied macro from tendermint-rpc crate because that's exactly what we have to do here too
macro_rules! perform_with_compat {
    ($self:expr, $request:expr) => {{
        let request = $request;
        match $self.compat {
            CompatMode::V0_38 => {
                $self
                    .perform_request_with_dialect(request, dialect::v0_38::Dialect)
                    .await
            }
            CompatMode::V0_37 => {
                $self
                    .perform_request_with_dialect(request, dialect::v0_37::Dialect)
                    .await
            }
            CompatMode::V0_34 => {
                $self
                    .perform_request_with_dialect(request, dialect::v0_34::Dialect)
                    .await
            }
        }
    }};
}

pub struct ReqwestRpcClient {
    compat: CompatMode,
    inner: Client,
    endpoint: Vec<String>,
    auth: Option<String>,
}

impl ReqwestRpcClient {
    pub fn new(url: Url) -> Self {
        let endpoint = url.path().split("/").map(Into::into).collect();
        let auth = extract_authorization(&url);
        ReqwestRpcClient {
            // after updating to nyxd 0.42 and thus updating to cometbft, the compat mode changed
            compat: CompatMode::V0_37,
            inner: Client::new(url, None),
            endpoint,
            auth,
        }
    }

    pub fn set_compat_mode(&mut self, compat: CompatMode) {
        self.compat = compat;
    }

    fn build_request<R, S>(&self, request: R) -> RequestBuilder
    where
        R: SimpleRequest<S>,
        S: Dialect,
    {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(
            header::USER_AGENT,
            format!("nym-reqwest-rpc-client/{}", env!("CARGO_PKG_VERSION"))
                .parse()
                .unwrap(),
        );
        if let Some(ref auth) = self.auth {
            headers.insert(header::AUTHORIZATION, auth.parse().unwrap());
        }

        let path_segments: Vec<_> = self.endpoint.iter().map(String::as_str).collect();
        self.inner
            .create_post_request(&path_segments, NO_PARAMS, &request)
            .headers(headers)
    }

    async fn perform_request_with_dialect<R, S>(
        &self,
        request: R,
        _dialect: S,
    ) -> Result<R::Output, Error>
    where
        R: SimpleRequest<S>,
        S: Dialect,
    {
        let request = self.build_request(request);
        // that's extremely unfortunate. the trait requires returning tendermint rpc error so we have to make best effort error mapping
        let response = request
            .send()
            .await
            .map_err(TendermintRpcErrorMap::into_rpc_err)?;
        let response_status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(TendermintRpcErrorMap::into_rpc_err)?;

        // Successful JSON-RPC requests are expected to return a 200 OK HTTP status.
        // Otherwise, this means that the HTTP request failed as a whole,
        // as opposed to the JSON-RPC request returning an error,
        // and we cannot expect the response body to be a valid JSON-RPC response.
        if response_status != reqwest::StatusCode::OK {
            // hehe, that's so nasty but we have to somehow convert between different versions of the same lib
            return Err(Error::http_request_failed(
                response_status.as_u16().try_into().unwrap(),
            ));
        }

        R::Response::from_string(bytes).map(Into::into)
    }
}

trait TendermintRpcErrorMap {
    fn into_rpc_err(self) -> Error;
}

impl TendermintRpcErrorMap for reqwest::Error {
    fn into_rpc_err(self) -> Error {
        todo!()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl TendermintRpcClient for ReqwestRpcClient {
    async fn perform<R>(&self, request: R) -> Result<R::Output, Error>
    where
        R: SimpleRequest,
    {
        self.perform_request_with_dialect(request, LatestDialect)
            .await
    }

    async fn block<H>(&self, height: H) -> Result<endpoint::block::Response, Error>
    where
        H: Into<Height> + Send,
    {
        perform_with_compat!(self, endpoint::block::Request::new(height.into()))
    }

    async fn block_by_hash(
        &self,
        hash: tendermint::Hash,
    ) -> Result<endpoint::block_by_hash::Response, Error> {
        perform_with_compat!(self, endpoint::block_by_hash::Request::new(hash))
    }

    async fn latest_block(&self) -> Result<endpoint::block::Response, Error> {
        perform_with_compat!(self, endpoint::block::Request::default())
    }

    async fn block_results<H>(&self, height: H) -> Result<endpoint::block_results::Response, Error>
    where
        H: Into<Height> + Send,
    {
        perform_with_compat!(self, endpoint::block_results::Request::new(height.into()))
    }

    async fn latest_block_results(&self) -> Result<endpoint::block_results::Response, Error> {
        perform_with_compat!(self, endpoint::block_results::Request::default())
    }

    async fn block_search(
        &self,
        query: Query,
        page: u32,
        per_page: u8,
        order: Order,
    ) -> Result<endpoint::block_search::Response, Error> {
        perform_with_compat!(
            self,
            endpoint::block_search::Request::new(query, page, per_page, order)
        )
    }

    async fn header<H>(&self, height: H) -> Result<endpoint::header::Response, Error>
    where
        H: Into<Height> + Send,
    {
        let height = height.into();
        match self.compat {
            CompatMode::V0_38 => {
                self.perform_request_with_dialect(
                    endpoint::header::Request::new(height),
                    v0_38::Dialect,
                )
                .await
            }
            CompatMode::V0_37 => {
                self.perform_request_with_dialect(
                    endpoint::header::Request::new(height),
                    v0_37::Dialect,
                )
                .await
            }
            CompatMode::V0_34 => {
                // Back-fill with a request to /block endpoint and
                // taking just the header from the response.
                let resp = self
                    .perform_request_with_dialect(block::Request::new(height), v0_34::Dialect)
                    .await?;
                Ok(resp.into())
            }
        }
    }

    async fn header_by_hash(&self, hash: Hash) -> Result<header_by_hash::Response, Error> {
        match self.compat {
            CompatMode::V0_38 => {
                self.perform_request_with_dialect(
                    header_by_hash::Request::new(hash),
                    v0_38::Dialect,
                )
                .await
            }
            CompatMode::V0_37 => {
                self.perform_request_with_dialect(
                    header_by_hash::Request::new(hash),
                    v0_37::Dialect,
                )
                .await
            }
            CompatMode::V0_34 => {
                // Back-fill with a request to /block_by_hash endpoint and
                // taking just the header from the response.
                let resp = self
                    .perform_request_with_dialect(block_by_hash::Request::new(hash), v0_34::Dialect)
                    .await?;
                Ok(resp.into())
            }
        }
    }

    /// `/broadcast_evidence`: broadcast an evidence.
    async fn broadcast_evidence(&self, e: Evidence) -> Result<evidence::Response, Error> {
        match self.compat {
            CompatMode::V0_38 => {
                self.perform_request_with_dialect(evidence::Request::new(e), v0_38::Dialect)
                    .await
            }
            CompatMode::V0_37 => {
                self.perform_request_with_dialect(evidence::Request::new(e), v0_37::Dialect)
                    .await
            }
            CompatMode::V0_34 => {
                self.perform_request_with_dialect(evidence::Request::new(e), v0_34::Dialect)
                    .await
            }
        }
    }

    async fn tx(&self, hash: Hash, prove: bool) -> Result<tx::Response, Error> {
        perform_with_compat!(self, tx::Request::new(hash, prove))
    }

    async fn tx_search(
        &self,
        query: Query,
        prove: bool,
        page: u32,
        per_page: u8,
        order: Order,
    ) -> Result<tx_search::Response, Error> {
        perform_with_compat!(
            self,
            tx_search::Request::new(query, prove, page, per_page, order)
        )
    }

    async fn broadcast_tx_commit<T>(&self, tx: T) -> Result<broadcast::tx_commit::Response, Error>
    where
        T: Into<Vec<u8>> + Send,
    {
        perform_with_compat!(self, broadcast::tx_commit::Request::new(tx))
    }
}

// essentially https://github.com/informalsystems/tendermint-rs/blob/v0.32.0/rpc/src/client/transport/auth.rs#L31
pub fn extract_authorization(url: &Url) -> Option<String> {
    if !url.has_authority() {
        return None;
    }

    let authority = url.authority();
    if let Some((userpass, _)) = authority.split_once('@') {
        Some(base64::prelude::BASE64_STANDARD.encode(userpass))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod extracting_url_authorization {
        use super::*;
        use std::str::FromStr;

        #[test]
        fn extract_auth_absent() {
            let uri = Url::from_str("http://example.com").unwrap();
            assert_eq!(extract_authorization(&uri), None);
        }

        #[test]
        fn extract_auth_username_only() {
            let uri = Url::from_str("http://toto@example.com").unwrap();
            let base64 = "dG90bw==".to_string();
            assert_eq!(extract_authorization(&uri), Some(base64));
        }

        #[test]
        fn extract_auth_username_password() {
            let uri = Url::from_str("http://toto:tata@example.com").unwrap();
            let base64 = "dG90bzp0YXRh".to_string();
            assert_eq!(extract_authorization(&uri), Some(base64));
        }
    }
}
