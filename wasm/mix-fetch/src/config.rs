// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// due to expansion of #[wasm_bindgen] macro on `MixFetch` Config struct
#![allow(clippy::drop_non_drop)]

use crate::error::MixFetchError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use wasm_client_core::config::{new_base_client_config, BaseClientConfig, ConfigDebug, DebugWasm};
use wasm_client_core::helpers::parse_recipient;
use wasm_client_core::Recipient;

const DEFAULT_MIX_FETCH_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_MIX_FETCH_ID: &str = "_default-nym-mix-fetch";
const MIX_FETCH_CLIENT_ID_PREFIX: &str = "mix-fetch";

fn make_mix_fetch_id(id: Option<String>) -> String {
    if let Some(provided) = id {
        format!("{MIX_FETCH_CLIENT_ID_PREFIX}-{provided}")
    } else {
        DEFAULT_MIX_FETCH_ID.to_string()
    }
}

#[wasm_bindgen]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixFetchConfig {
    pub(crate) base: BaseClientConfig,

    pub(crate) mix_fetch: MixFetch,
}

#[derive(Tsify, Debug, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct MixFetchConfigOpts {
    #[tsify(optional)]
    pub id: Option<String>,

    #[tsify(optional)]
    pub nym_api: Option<String>,

    #[tsify(optional)]
    pub nyxd: Option<String>,

    #[tsify(optional)]
    pub debug: Option<DebugWasm>,
}

#[wasm_bindgen]
impl MixFetchConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(
        network_requester_address: String,
        opts: Option<MixFetchConfigOpts>,
    ) -> Result<MixFetchConfig, MixFetchError> {
        let version = env!("CARGO_PKG_VERSION").to_string();
        if let Some(opts) = opts {
            Ok(MixFetchConfig {
                base: new_base_client_config(
                    make_mix_fetch_id(opts.id),
                    version,
                    opts.nym_api,
                    opts.nyxd,
                    opts.debug,
                )?,
                mix_fetch: MixFetch::new(network_requester_address)?,
            })
        } else {
            Ok(MixFetchConfig {
                base: BaseClientConfig::new(make_mix_fetch_id(None), version),
                mix_fetch: MixFetch::new(network_requester_address)?,
            })
        }
    }
}

#[wasm_bindgen]
impl MixFetchConfig {
    pub fn with_mix_fetch_timeout(mut self, timeout_ms: u32) -> Self {
        self.mix_fetch.debug.request_timeout = Duration::from_millis(timeout_ms as u64);
        self
    }
}

impl MixFetchConfig {
    pub fn override_debug<D: Into<ConfigDebug>>(&mut self, debug: D) {
        self.base.debug = debug.into();
    }

    pub fn override_mix_fetch_debug<D: Into<MixFetchDebug>>(&mut self, debug: D) {
        self.mix_fetch.debug = debug.into();
    }
}

#[wasm_bindgen]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixFetch {
    pub(crate) network_requester_address: Recipient,

    #[serde(default)]
    pub(crate) debug: MixFetchDebug,
}

impl MixFetch {
    pub(crate) fn new(network_requester_address: String) -> Result<MixFetch, MixFetchError> {
        Ok(MixFetch {
            network_requester_address: parse_recipient(&network_requester_address)?,
            debug: Default::default(),
        })
    }
}

#[wasm_bindgen]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixFetchDebug {
    pub(crate) request_timeout: Duration,
}

impl Default for MixFetchDebug {
    fn default() -> Self {
        MixFetchDebug {
            request_timeout: DEFAULT_MIX_FETCH_TIMEOUT,
        }
    }
}

#[derive(Tsify, Debug, Copy, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct MixFetchDebugOverride {
    #[tsify(optional)]
    pub request_timeout_ms: Option<u32>,
}

impl From<MixFetchDebugOverride> for MixFetchDebug {
    fn from(value: MixFetchDebugOverride) -> Self {
        let def = MixFetchDebug::default();

        MixFetchDebug {
            request_timeout: value
                .request_timeout_ms
                .map(|d| Duration::from_millis(d as u64))
                .unwrap_or(def.request_timeout),
        }
    }
}
