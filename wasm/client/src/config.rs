// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmClientError;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_client_core::config::{new_base_client_config, BaseClientConfig, DebugWasm};

#[wasm_bindgen]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClientConfig {
    pub(crate) base: BaseClientConfig,
}

#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfigOpts {
    #[serde(rename = "nymApi")]
    nym_api: Option<String>,
    nyxd: Option<String>,
    debug: Option<DebugWasm>,
}

#[wasm_bindgen]
impl ClientConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(id: String, opts: JsValue) -> Result<ClientConfig, WasmClientError> {
        let opts = if opts.is_null() || opts.is_undefined() {
            None
        } else {
            Some(serde_wasm_bindgen::from_value(opts)?)
        };
        ClientConfig::_new(id, opts)
    }

    pub(crate) fn _new(
        id: String,
        opts: Option<ClientConfigOpts>,
    ) -> Result<ClientConfig, WasmClientError> {
        let version = env!("CARGO_PKG_VERSION").to_string();
        if let Some(opts) = opts {
            Ok(ClientConfig {
                base: new_base_client_config(id, version, opts.nym_api, opts.nyxd, opts.debug)?,
            })
        } else {
            Ok(ClientConfig {
                base: BaseClientConfig::new(id, version),
            })
        }
    }

    #[cfg(feature = "node-tester")]
    pub(crate) fn new_tester_config<S: Into<String>>(id: S) -> Self {
        ClientConfig {
            base: BaseClientConfig::new(id.into(), env!("CARGO_PKG_VERSION").to_string())
                .with_disabled_credentials(true)
                .with_disabled_cover_traffic(true)
                .with_disabled_topology_refresh(true),
        }
    }
}
