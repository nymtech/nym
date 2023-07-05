// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// due to expansion of #[wasm_bindgen] macro on `MixFetch` Config struct
#![allow(clippy::drop_non_drop)]

use crate::config::base_wasm::{new_base_client, DebugWasm};
use crate::error::WasmClientError;
use crate::helpers::parse_recipient;
use nym_client_core::config::Config as BaseClientConfig;
use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use wasm_bindgen::prelude::*;

const DEFAULT_MIX_FETCH_TIMEOUT: Duration = Duration::from_secs(5);

#[wasm_bindgen]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixFetchConfig {
    pub(crate) base: BaseClientConfig,

    pub(crate) mix_fetch: MixFetch,
}

#[wasm_bindgen]
impl MixFetchConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: String,
        network_requester_address: String,
        nym_api: Option<String>,
        nyxd: Option<String>,
        debug: Option<DebugWasm>,
    ) -> Result<MixFetchConfig, WasmClientError> {
        Ok(MixFetchConfig {
            base: new_base_client(
                id,
                env!("CARGO_PKG_VERSION").to_string(),
                nym_api,
                nyxd,
                debug,
            )?,
            mix_fetch: MixFetch::new(network_requester_address)?,
        })
    }
}

#[wasm_bindgen]
impl MixFetchConfig {
    pub fn with_mix_fetch_timeout(mut self, timeout_ms: u64) -> Self {
        self.mix_fetch.request_timeout = Duration::from_millis(timeout_ms);
        self
    }
}

#[wasm_bindgen]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixFetch {
    pub(crate) network_requester_address: Recipient,

    pub(crate) request_timeout: Duration,
}

impl MixFetch {
    pub(crate) fn new(network_requester_address: String) -> Result<MixFetch, WasmClientError> {
        Ok(MixFetch {
            network_requester_address: parse_recipient(&network_requester_address)?,
            request_timeout: DEFAULT_MIX_FETCH_TIMEOUT,
        })
    }
}
