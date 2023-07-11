// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::config::GatewayEndpointConfig;
use nym_crypto::asymmetric::{encryption, identity};
use nym_topology::gateway::GatewayConversionError;
use nym_topology::mix::{Layer, MixnodeConversionError};
use nym_topology::{gateway, mix, MixLayer, NymTopology};
use nym_validator_client::client::{IdentityKeyRef, MixId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wasm_utils::console_log;
use wasm_utils::error::simple_js_error;

#[derive(Debug, Error)]
pub enum WasmTopologyError {
    #[error("got invalid mix layer {value}. Expected 1, 2 or 3.")]
    InvalidMixLayer { value: u8 },

    #[error(transparent)]
    GatewayConversion(#[from] GatewayConversionError),

    #[error(transparent)]
    MixnodeConversion(#[from] MixnodeConversionError),

    #[error("The provided mixnode map was malformed: {msg}")]
    MalformedMixnodeMap { msg: String },

    #[error("The provided gateway list was malformed: {msg}")]
    MalformedGatewayList { msg: String },
}

impl From<WasmTopologyError> for JsValue {
    fn from(value: WasmTopologyError) -> Self {
        simple_js_error(value.to_string())
    }
}

// serde helper, not intended to be used directly
#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmNymTopology {
    mixnodes: BTreeMap<MixLayer, Vec<WasmMixNode>>,
    gateways: Vec<WasmGateway>,
}

#[wasm_bindgen]
impl WasmNymTopology {
    // blame javascript on that nasty constructor...
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(mixnodes: JsValue, gateways: JsValue) -> Result<JsValue, WasmTopologyError> {
        let mixnodes: BTreeMap<MixLayer, Vec<WasmMixNode>> =
            serde_wasm_bindgen::from_value(mixnodes).map_err(|source| {
                WasmTopologyError::MalformedMixnodeMap {
                    msg: source.to_string(),
                }
            })?;

        let gateways: Vec<WasmGateway> =
            serde_wasm_bindgen::from_value(gateways).map_err(|source| {
                WasmTopologyError::MalformedGatewayList {
                    msg: source.to_string(),
                }
            })?;
        let topology = WasmNymTopology { mixnodes, gateways };

        // the unwrap is fine as we've just constructed the proper structs
        Ok(serde_wasm_bindgen::to_value(&topology).unwrap())
    }

    pub fn print(&self) {
        if !self.mixnodes.is_empty() {
            console_log!("mixnodes:");
            for (layer, nodes) in &self.mixnodes {
                console_log!("\tlayer {layer}:");
                for node in nodes {
                    console_log!("\t\t{} - {}", node.mix_id, node.identity_key)
                }
            }
        } else {
            console_log!("NO MIXNODES")
        }

        if !self.gateways.is_empty() {
            console_log!("gateways:");
            for gateway in &self.gateways {
                console_log!("\t{}", gateway.identity_key)
            }
        } else {
            console_log!("NO GATEWAYS")
        }
    }
}

impl TryFrom<WasmNymTopology> for NymTopology {
    type Error = WasmTopologyError;

    fn try_from(value: WasmNymTopology) -> Result<Self, Self::Error> {
        let mut converted_mixes = BTreeMap::new();

        for (layer, nodes) in value.mixnodes {
            let layer_nodes = nodes
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?;

            converted_mixes.insert(layer, layer_nodes);
        }

        let gateways = value
            .gateways
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()?;

        Ok(NymTopology::new(converted_mixes, gateways))
    }
}

impl WasmNymTopology {
    pub fn ensure_contains(&self, gateway_config: &GatewayEndpointConfig) -> bool {
        self.ensure_contains_gateway_id(&gateway_config.gateway_id)
    }

    pub fn ensure_contains_gateway_id(&self, gateway_id: IdentityKeyRef) -> bool {
        self.gateways.iter().any(|g| g.identity_key == gateway_id)
    }
}

impl From<NymTopology> for WasmNymTopology {
    fn from(value: NymTopology) -> Self {
        WasmNymTopology {
            mixnodes: value
                .mixes()
                .iter()
                .map(|(&l, nodes)| (l, nodes.iter().map(Into::into).collect()))
                .collect(),
            gateways: value.gateways().iter().map(Into::into).collect(),
        }
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

impl<'a> From<&'a mix::Node> for WasmMixNode {
    fn from(value: &'a mix::Node) -> Self {
        WasmMixNode {
            mix_id: value.mix_id,
            owner: value.owner.clone(),
            host: value.host.to_string(),
            mix_port: value.mix_host.port(),
            identity_key: value.identity_key.to_base58_string(),
            sphinx_key: value.sphinx_key.to_base58_string(),
            layer: value.layer.into(),
            version: value.version.clone(),
        }
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

impl<'a> From<&'a gateway::Node> for WasmGateway {
    fn from(value: &'a gateway::Node) -> Self {
        WasmGateway {
            owner: value.owner.clone(),
            host: value.host.to_string(),
            mix_port: value.mix_host.port(),
            clients_port: value.clients_port,
            identity_key: value.identity_key.to_base58_string(),
            sphinx_key: value.sphinx_key.to_base58_string(),
            version: value.version.clone(),
        }
    }
}
