// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::base_wasm::{new_base_client, DebugWasm};
use crate::error::WasmClientError;
use nym_client_core::config::Config as BaseClientConfig;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub(crate) base: BaseClientConfig,
}

#[wasm_bindgen]
impl Config {
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: String,
        nym_api: String,
        nyxd: String,
        debug: Option<DebugWasm>,
    ) -> Result<Config, WasmClientError> {
        Ok(Config {
            base: new_base_client(
                id,
                env!("CARGO_PKG_VERSION").to_string(),
                Some(nym_api),
                Some(nyxd),
                debug,
            )?,
        })
    }

    pub(crate) fn new_tester_config<S: Into<String>>(id: S) -> Self {
        Config {
            base: BaseClientConfig::new(id.into(), env!("CARGO_PKG_VERSION").to_string())
                .with_disabled_credentials(true)
                .with_disabled_cover_traffic(true)
                .with_disabled_topology_refresh(true),
        }
    }
}
