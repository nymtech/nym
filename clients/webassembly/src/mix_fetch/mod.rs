// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::client::MixFetchClient;
use crate::mix_fetch::config::MixFetchConfig;
use crate::mix_fetch::error::MixFetchError;
use js_sys::Promise;
use nym_validator_client::client::IdentityKey;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_log, PromisableResultError};

mod active_requests;
mod client;
mod config;
pub mod error;
mod go_bridge;
mod request_writer;
mod socks_helpers;

pub type RequestId = u64;

static MIX_FETCH: OnceLock<MixFetchClient> = OnceLock::new();

#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixFetchOpts {
    #[serde(rename = "preferredGateway")]
    preferred_gateway: Option<IdentityKey>,
    #[serde(rename = "storagePassphrase")]
    storage_passphrase: Option<String>,
}

#[wasm_bindgen(js_name = setupMixFetch)]
pub fn setup_mix_fetch(config: MixFetchConfig, opts: JsValue) -> Promise {
    if MIX_FETCH.get().is_some() {
        return MixFetchError::AlreadyInitialised.into_rejected_promise();
    }

    let opts = if opts.is_null() || opts.is_undefined() {
        None
    } else {
        match serde_wasm_bindgen::from_value(opts) {
            Ok(opts) => Some(opts),
            Err(err) => return MixFetchError::from(err).into_rejected_promise(),
        }
    };

    future_to_promise(async move {
        setup_mix_fetch_async(config, opts)
            .await
            .map(|_| JsValue::undefined())
            .map_promise_err()
    })
}

#[wasm_bindgen(js_name = setupMixFetchSimple)]
pub fn setup_mix_fetch_simple(network_requester_address: String) -> Promise {
    if MIX_FETCH.get().is_some() {
        return MixFetchError::AlreadyInitialised.into_rejected_promise();
    }

    let config = match MixFetchConfig::_new(network_requester_address, None) {
        Ok(config) => config,
        Err(err) => return err.into_rejected_promise(),
    };
    future_to_promise(async move {
        setup_mix_fetch_async(config, None)
            .await
            .map(|_| JsValue::undefined())
            .map_promise_err()
    })
}

fn set_mix_fetch_client(mix_fetch_client: MixFetchClient) -> Result<(), MixFetchError> {
    MIX_FETCH
        .set(mix_fetch_client)
        .map_err(|_| MixFetchError::AlreadyInitialised)
}

fn mix_fetch_client() -> Result<&'static MixFetchClient, MixFetchError> {
    MIX_FETCH.get().ok_or(MixFetchError::Uninitialised)
}

async fn setup_mix_fetch_async(
    config: MixFetchConfig,
    opts: Option<MixFetchOpts>,
) -> Result<(), MixFetchError> {
    console_log!("config: \n{config:#?}\n\nopts:\n{opts:#?}");

    let client = if let Some(opts) = opts {
        let preferred_gateway = opts.preferred_gateway;
        let storage_passphrase = opts.storage_passphrase;
        MixFetchClient::new_async(config, preferred_gateway, storage_passphrase).await?
    } else {
        MixFetchClient::new_async(config, None, None).await?
    };
    set_mix_fetch_client(client)?;
    Ok(())
}
