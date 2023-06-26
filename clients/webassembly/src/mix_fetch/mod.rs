// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::client::MixFetchClient;
use crate::mix_fetch::config::MixFetchConfig;
use crate::mix_fetch::error::MixFetchError;
use js_sys::Promise;
use nym_validator_client::client::IdentityKey;
use std::sync::OnceLock;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_log, PromisableResult};

mod active_requests;
mod client;
mod config;
pub mod error;
mod go_bridge;
mod request_writer;
mod socks_helpers;

pub type RequestId = u64;

pub const MIX_FETCH_CLIENT_ID_PREFIX: &str = "mix-fetch-";

static MIX_FETCH: OnceLock<MixFetchClient> = OnceLock::new();

fn set_mix_fetch_client(mix_fetch_client: MixFetchClient) -> Result<(), MixFetchError> {
    MIX_FETCH
        .set(mix_fetch_client)
        .map_err(|_| MixFetchError::AlreadyInitialised)
}

fn mix_fetch_client() -> Result<&'static MixFetchClient, MixFetchError> {
    MIX_FETCH.get().ok_or(MixFetchError::Uninitialised)
}

#[wasm_bindgen(js_name = setupMixFetch)]
pub fn setup_mix_fetch(
    config: MixFetchConfig,
    preferred_gateway: Option<IdentityKey>,
    storage_passphrase: Option<String>,
) -> Promise {
    if MIX_FETCH.get().is_some() {
        MixFetchError::AlreadyInitialised.into_rejected_promise()
    } else {
        future_to_promise(async move {
            let client =
                MixFetchClient::new_async(config, preferred_gateway, storage_passphrase).await?;
            set_mix_fetch_client(client)?;
            Ok(JsValue::undefined())
        })
    }
}

// https://developer.mozilla.org/en-US/docs/Web/API/fetch#syntax
#[wasm_bindgen(js_name = mixFetch)]
pub fn mix_fetch(resource: JsValue, options: Option<web_sys::RequestInit>) -> Promise {
    future_to_promise(async move {
        mix_fetch_async(resource, options)
            .await
            .into_promise_result()
    })
}

#[wasm_bindgen(js_name = isInitialised)]
pub fn mix_fetch_initialised() -> bool {
    MIX_FETCH.get().is_some()
}

#[derive(Debug)]
pub enum Resource {
    Url(url::Url),
    Request(web_sys::Request),
}

impl Resource {
    fn to_request(
        &self,
        options: Option<web_sys::RequestInit>,
    ) -> Result<web_sys::Request, JsValue> {
        match self {
            Resource::Url(url) => {
                if let Some(options) = options {
                    web_sys::Request::new_with_str_and_init(url.as_str(), &options)
                } else {
                    web_sys::Request::new_with_str(url.as_str())
                }
            }
            Resource::Request(request) => {
                if let Some(options) = options {
                    web_sys::Request::new_with_request_and_init(request, &options)
                } else {
                    web_sys::Request::new_with_request(request)
                }
            }
        }
    }
}

impl TryFrom<JsValue> for Resource {
    type Error = MixFetchError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        if value.is_string() {
            let string = value
                .as_string()
                .ok_or(MixFetchError::NotStringMixFetchUrl)?;
            Ok(Resource::Url(string.parse()?))
        } else {
            Ok(Resource::Request(web_sys::Request::from(value)))
        }
    }
}

async fn mix_fetch_async(
    resource: JsValue,
    options: Option<web_sys::RequestInit>,
) -> Result<web_sys::Response, JsValue> {
    let resource = Resource::try_from(resource)?;
    console_log!("mix fetch with {resource:?} and {options:?}");

    let request = resource.to_request(options)?;
    let mix_fetch_client = mix_fetch_client()?;
    mix_fetch_client.fetch_async2(request).await
    // mix_fetch_client.fetch_async(request).await
}
