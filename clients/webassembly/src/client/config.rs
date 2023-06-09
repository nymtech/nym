// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// due to expansion of #[wasm_bindgen] macro on `Debug` Config struct
#![allow(clippy::drop_non_drop)]
// another issue due to #[wasm_bindgen] and `Copy` trait
#![allow(clippy::drop_copy)]

use nym_client_core::config::{
    Acknowledgements as ConfigAcknowledgements, Config as BaseClientConfig,
    CoverTraffic as ConfigCoverTraffic, DebugConfig as ConfigDebug,
    GatewayConnection as ConfigGatewayConnection, ReplySurbs as ConfigReplySurbs,
    Topology as ConfigTopology, Traffic as ConfigTraffic,
};
use nym_sphinx::params::{PacketSize, PacketType};
use serde::{Deserialize, Serialize};
use std::time::Duration;
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
    pub fn new(id: String, validator_server: String, debug: Option<DebugWasm>) -> Self {
        Config {
            base: BaseClientConfig::new(id, env!("CARGO_PKG_VERSION").to_string())
                .with_custom_nyxd(vec![validator_server
                    .parse()
                    .expect("provided url was malformed")])
                .with_debug_config(debug.map(Into::into).unwrap_or_default()),
        }
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

#[wasm_bindgen]
pub fn default_debug() -> DebugWasm {
    ConfigDebug::default().into()
}
