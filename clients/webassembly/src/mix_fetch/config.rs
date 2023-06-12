// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::base_wasm::{new_base_client, DebugWasm};
use crate::error::WasmClientError;
use crate::helpers::parse_recipient;
use nym_client_core::config::Config as BaseClientConfig;
use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

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
            base: new_base_client(id, nym_api, nyxd, debug)?,
            mix_fetch: MixFetch::new(network_requester_address)?,
        })
    }
}

#[wasm_bindgen]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixFetch {
    pub(crate) network_requester_address: Recipient,
}

impl MixFetch {
    pub(crate) fn new(network_requester_address: String) -> Result<MixFetch, WasmClientError> {
        Ok(MixFetch {
            network_requester_address: parse_recipient(&network_requester_address)?,
        })
    }
}
