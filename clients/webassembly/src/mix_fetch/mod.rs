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
