// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::config::GatewayEndpointConfig;
use nym_crypto::asymmetric::{encryption, identity};
use nym_topology::gateway::GatewayConversionError;
use nym_topology::mix::{Layer, MixnodeConversionError};
use nym_topology::{gateway, mix, MixLayer, NymTopology};
use nym_validator_client::client::MixId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_utils::{console_log, simple_js_error};

#[derive(Debug, Error)]
pub enum WasmTopologyError {
    #[error("got invalid mix layer {value}. Expected 1, 2 or 3.")]
    InvalidMixLayer { value: u8 },

    #[error(transparent)]
    GatewayConversion(#[from] GatewayConversionError),

    #[error(transparent)]
    MixnodeConversion(#[from] MixnodeConversionError),
}

impl From<WasmTopologyError> for JsValue {
    fn from(value: WasmTopologyError) -> Self {
        simple_js_error(value.to_string())
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct WasmNymTopology {
    inner: NymTopology,
}

#[wasm_bindgen]
impl WasmNymTopology {
    #[wasm_bindgen(constructor)]
    pub fn new(
        // expected: BTreeMap<MixLayer, Vec<WasmMixNode>>,
        // HashMap<MixLayer, Vec<WasmMixNode>> will also work because it has the same json representation
        mixnodes: JsValue,
        // expected: Vec<WasmGateway>
        gateways: JsValue,
    ) -> Result<WasmNymTopology, WasmTopologyError> {
        let mixnodes: BTreeMap<MixLayer, Vec<WasmMixNode>> =
            serde_wasm_bindgen::from_value(mixnodes).expect("TODO");

        let gateways: Vec<WasmGateway> = serde_wasm_bindgen::from_value(gateways).expect("TODO");

        let mut converted_mixes = BTreeMap::new();

        for (layer, nodes) in mixnodes {
            let layer_nodes = nodes
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?;

            converted_mixes.insert(layer, layer_nodes);
        }

        let gateways = gateways
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()?;

        Ok(WasmNymTopology {
            inner: NymTopology::new(converted_mixes, gateways),
        })
    }

    pub(crate) fn ensure_contains(&self, gateway_config: &GatewayEndpointConfig) -> bool {
        self.inner
            .gateways()
            .iter()
            .any(|g| g.identity_key.to_base58_string() == gateway_config.gateway_id)
    }

    pub fn print(&self) {
        if !self.inner.mixes().is_empty() {
            console_log!("mixnodes:");
            for (layer, nodes) in self.inner.mixes() {
                console_log!("\tlayer {layer}:");
                for node in nodes {
                    console_log!("\t\t{} - {}", node.mix_id, node.identity_key)
                }
            }
        } else {
            console_log!("NO MIXNODES")
        }

        if !self.inner.gateways().is_empty() {
            console_log!("gateways:");
            for gateway in self.inner.gateways() {
                console_log!("\t{}", gateway.identity_key)
            }
        } else {
            console_log!("NO GATEWAYS")
        }
    }
}

impl From<WasmNymTopology> for NymTopology {
    fn from(value: WasmNymTopology) -> Self {
        value.inner
    }
}

impl From<NymTopology> for WasmNymTopology {
    fn from(value: NymTopology) -> Self {
        WasmNymTopology { inner: value }
    }
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WasmMixNode {
    pub mix_id: MixId,
    #[wasm_bindgen(getter_with_clone)]
    pub owner: String,
    #[wasm_bindgen(getter_with_clone)]
    pub host: String,
    pub mix_port: u16,
    #[wasm_bindgen(getter_with_clone)]
    pub identity_key: String,
    #[wasm_bindgen(getter_with_clone)]
    pub sphinx_key: String,
    pub layer: MixLayer,
    #[wasm_bindgen(getter_with_clone)]
    pub version: String,
}

#[wasm_bindgen]
impl WasmMixNode {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mix_id: MixId,
        owner: String,
        host: String,
        mix_port: u16,
        identity_key: String,
        sphinx_key: String,
        layer: MixLayer,
        version: String,
    ) -> Self {
        Self {
            mix_id,
            owner,
            host,
            mix_port,
            identity_key,
            sphinx_key,
            layer,
            version,
        }
    }
}

impl TryFrom<WasmMixNode> for mix::Node {
    type Error = WasmTopologyError;

    fn try_from(value: WasmMixNode) -> Result<Self, Self::Error> {
        let host = mix::Node::parse_host(&value.host)?;

        // try to completely resolve the host in the mix situation to avoid doing it every
        // single time we want to construct a path
        let mix_host = mix::Node::extract_mix_host(&host, value.mix_port)?;

        Ok(mix::Node {
            mix_id: value.mix_id,
            owner: value.owner,
            host,
            mix_host,
            identity_key: identity::PublicKey::from_base58_string(&value.identity_key)
                .map_err(MixnodeConversionError::from)?,
            sphinx_key: encryption::PublicKey::from_base58_string(&value.sphinx_key)
                .map_err(MixnodeConversionError::from)?,
            layer: Layer::try_from(value.layer)
                .map_err(|_| WasmTopologyError::InvalidMixLayer { value: value.layer })?,
            version: value.version,
        })
    }
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WasmGateway {
    #[wasm_bindgen(getter_with_clone)]
    pub owner: String,
    #[wasm_bindgen(getter_with_clone)]
    pub host: String,
    pub mix_port: u16,
    pub clients_port: u16,
    #[wasm_bindgen(getter_with_clone)]
    pub identity_key: String,
    #[wasm_bindgen(getter_with_clone)]
    pub sphinx_key: String,
    #[wasm_bindgen(getter_with_clone)]
    pub version: String,
}

#[wasm_bindgen]
impl WasmGateway {
    #[wasm_bindgen(constructor)]
    pub fn new(
        owner: String,
        host: String,
        mix_port: u16,
        clients_port: u16,
        identity_key: String,
        sphinx_key: String,
        version: String,
    ) -> Self {
        Self {
            owner,
            host,
            mix_port,
            clients_port,
            identity_key,
            sphinx_key,
            version,
        }
    }
}

impl TryFrom<WasmGateway> for gateway::Node {
    type Error = WasmTopologyError;

    fn try_from(value: WasmGateway) -> Result<Self, Self::Error> {
        let host = gateway::Node::parse_host(&value.host)?;

        // try to completely resolve the host in the mix situation to avoid doing it every
        // single time we want to construct a path
        let mix_host = gateway::Node::extract_mix_host(&host, value.mix_port)?;

        Ok(gateway::Node {
            owner: value.owner,
            host,
            mix_host,
            clients_port: value.clients_port,
            identity_key: identity::PublicKey::from_base58_string(&value.identity_key)
                .map_err(GatewayConversionError::from)?,
            sphinx_key: encryption::PublicKey::from_base58_string(&value.sphinx_key)
                .map_err(GatewayConversionError::from)?,
            version: value.version,
        })
    }
}
