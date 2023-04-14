// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmClientError;
use crate::topology::WasmNymTopology;
use js_sys::Promise;
use nym_client_core::client::key_manager::KeyManager;
use nym_client_core::client::replies::reply_storage::browser_backend;
use nym_client_core::config;
use nym_topology::NymTopology;
use nym_validator_client::NymApiClient;
use rand::rngs::OsRng;
use url::Url;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::console_log;

pub(crate) fn setup_new_key_manager() -> KeyManager {
    let mut rng = OsRng;
    console_log!("generated new set of keys");
    KeyManager::new(&mut rng)
}

// don't get too excited about the name, under the hood it's just a big fat placeholder
// with no persistence
pub(crate) fn setup_reply_surb_storage_backend(
    config: config::ReplySurbs,
) -> browser_backend::Backend {
    browser_backend::Backend::new(
        config.minimum_reply_surb_storage_threshold,
        config.maximum_reply_surb_storage_threshold,
    )
}

pub(crate) async fn current_network_topology_async(
    nym_api_url: String,
) -> Result<WasmNymTopology, WasmClientError> {
    let url: Url = match nym_api_url.parse() {
        Ok(url) => url,
        Err(source) => {
            return Err(WasmClientError::MalformedUrl {
                raw: nym_api_url,
                source,
            })
        }
    };

    let api_client = NymApiClient::new(url);
    let mixnodes = api_client.get_cached_active_mixnodes().await?;
    let gateways = api_client.get_cached_gateways().await?;

    Ok(NymTopology::from_detailed(mixnodes, gateways).into())
}

#[wasm_bindgen]
pub fn current_network_topology(nym_api_url: String) -> Promise {
    future_to_promise(async move {
        match current_network_topology_async(nym_api_url).await {
            Ok(topology) => Ok(JsValue::from(topology)),
            Err(err) => Err(err.into()),
        }
    })
}
