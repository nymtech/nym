// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmClientError;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use wasm_client_core::config::{new_base_client_config, BaseClientConfig, DebugWasm};

#[wasm_bindgen]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClientConfig {
    pub(crate) base: BaseClientConfig,
}

#[derive(Tsify, Debug, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct ClientConfigOpts {
    #[tsify(optional)]
    nym_api: Option<String>,

    #[tsify(optional)]
    nyxd: Option<String>,

    #[tsify(optional)]
    debug: Option<DebugWasm>,
}

#[wasm_bindgen]
impl ClientConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(id: String, opts: ClientConfigOpts) -> Result<ClientConfig, WasmClientError> {
        let version = env!("CARGO_PKG_VERSION").to_string();

        Ok(ClientConfig {
            base: new_base_client_config(id, version, opts.nym_api, opts.nyxd, opts.debug)?,
        })
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
