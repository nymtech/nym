// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::rpc::TendermintRpcClient;
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use reqwest::{header, RequestBuilder};
use tendermint_rpc::{Error, Response, SimpleRequest};
use url::Url;

pub struct ReqwestRpcClient {
    inner: reqwest::Client,
    url: Url,
}

impl ReqwestRpcClient {
    pub fn new(url: Url) -> Self {
        ReqwestRpcClient {
            inner: reqwest::Client::new(),
            url,
        }
    }

    fn build_request<R: SimpleRequest>(&self, request: R) -> RequestBuilder {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(
            header::USER_AGENT,
            format!("nym-reqwest-rpc-client/{}", env!("CARGO_PKG_VERSION"))
                .parse()
                .unwrap(),
        );
        if let Some(auth) = extract_authorization(&self.url) {
            headers.insert(header::AUTHORIZATION, auth.parse().unwrap());
        }

        self.inner
            .post(self.url.clone())
            .body(request.into_json().into_bytes())
            .headers(headers)
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
        let request = self.build_request(request);
        // that's extremely unfortunate. the trait requires returning tendermint rpc error so we have to make best effort error mapping
        let response = request.send().await.unwrap();
        let bytes = response.bytes().await.unwrap();
        R::Response::from_string(bytes).map(Into::into)
    }
}

// essentially https://github.com/informalsystems/tendermint-rs/blob/v0.32.0/rpc/src/client/transport/auth.rs#L31
pub fn extract_authorization(url: &Url) -> Option<String> {
    if !url.has_authority() {
        return None;
    }

    let authority = url.authority();
    if let Some((userpass, _)) = authority.split_once('@') {
        Some(base64::encode(userpass))
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
