// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::config::GatewayEndpointConfig;
use nym_config::defaults::{DEFAULT_CLIENT_LISTENING_PORT, DEFAULT_MIX_LISTENING_PORT};
use nym_crypto::asymmetric::{encryption, identity};
use nym_topology::gateway::GatewayConversionError;
use nym_topology::mix::{Layer, MixnodeConversionError};
use nym_topology::{gateway, mix, MixLayer, NymTopology};
use nym_validator_client::client::IdentityKeyRef;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use tsify::Tsify;
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
#[derive(Tsify, Debug, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct WasmNymTopology {
    mixnodes: BTreeMap<MixLayer, Vec<WasmMixNode>>,
    gateways: Vec<WasmGateway>,
}

impl WasmNymTopology {
    pub fn print(&self) {
        if !self.mixnodes.is_empty() {
            console_log!("mixnodes:");
            for (layer, nodes) in &self.mixnodes {
                console_log!("\tlayer {layer}:");
                for node in nodes {
                    // console_log!("\t\t{} - {}", node.mix_id, node.identity_key)
                    console_log!("\t\t{} - {:?}", node.mix_id, node)
                }
            }
        } else {
            console_log!("NO MIXNODES")
        }

        if !self.gateways.is_empty() {
            console_log!("gateways:");
            for gateway in &self.gateways {
                // console_log!("\t{}", gateway.identity_key)
                console_log!("\t{:?}", gateway)
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

#[derive(Tsify, Debug, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct WasmMixNode {
    // this is a `MixId` but due to typescript issue, we're using u32 directly.
    pub mix_id: u32,
    pub owner: String,
    pub host: String,

    #[tsify(optional)]
    pub mix_port: Option<u16>,
    pub identity_key: String,
    pub sphinx_key: String,

    // this is a `MixLayer` but due to typescript issue, we're using u8 directly.
    pub layer: u8,

    #[tsify(optional)]
    pub version: Option<String>,
}

impl TryFrom<WasmMixNode> for mix::Node {
    type Error = WasmTopologyError;

    fn try_from(value: WasmMixNode) -> Result<Self, Self::Error> {
        let host = mix::Node::parse_host(&value.host)?;

        let mix_port = value.mix_port.unwrap_or(DEFAULT_MIX_LISTENING_PORT);
        let version = value.version.map(|v| v.as_str().into()).unwrap_or_default();

        // try to completely resolve the host in the mix situation to avoid doing it every
        // single time we want to construct a path
        let mix_host = mix::Node::extract_mix_host(&host, mix_port)?;

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
            version,
        })
    }
}

impl<'a> From<&'a mix::Node> for WasmMixNode {
    fn from(value: &'a mix::Node) -> Self {
        WasmMixNode {
            mix_id: value.mix_id,
            owner: value.owner.clone(),
            host: value.host.to_string(),
            mix_port: Some(value.mix_host.port()),
            identity_key: value.identity_key.to_base58_string(),
            sphinx_key: value.sphinx_key.to_base58_string(),
            layer: value.layer.into(),
            version: Some(value.version.to_string()),
        }
    }
}

#[derive(Tsify, Debug, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct WasmGateway {
    pub owner: String,
    pub host: String,

    #[tsify(optional)]
    pub mix_port: Option<u16>,

    #[tsify(optional)]
    pub clients_port: Option<u16>,
    pub identity_key: String,
    pub sphinx_key: String,

    #[tsify(optional)]
    pub version: Option<String>,
}

impl TryFrom<WasmGateway> for gateway::Node {
    type Error = WasmTopologyError;

    fn try_from(value: WasmGateway) -> Result<Self, Self::Error> {
        let host = gateway::Node::parse_host(&value.host)?;

        let mix_port = value.mix_port.unwrap_or(DEFAULT_MIX_LISTENING_PORT);
        let clients_port = value.clients_port.unwrap_or(DEFAULT_CLIENT_LISTENING_PORT);
        let version = value.version.map(|v| v.as_str().into()).unwrap_or_default();

        // try to completely resolve the host in the mix situation to avoid doing it every
        // single time we want to construct a path
        let mix_host = gateway::Node::extract_mix_host(&host, mix_port)?;

        Ok(gateway::Node {
            owner: value.owner,
            host,
            mix_host,
            clients_port,
            identity_key: identity::PublicKey::from_base58_string(&value.identity_key)
                .map_err(GatewayConversionError::from)?,
            sphinx_key: encryption::PublicKey::from_base58_string(&value.sphinx_key)
                .map_err(GatewayConversionError::from)?,
            version,
        })
    }
}

impl<'a> From<&'a gateway::Node> for WasmGateway {
    fn from(value: &'a gateway::Node) -> Self {
        WasmGateway {
            owner: value.owner.clone(),
            host: value.host.to_string(),
            mix_port: Some(value.mix_host.port()),
            clients_port: Some(value.clients_port),
            identity_key: value.identity_key.to_base58_string(),
            sphinx_key: value.sphinx_key.to_base58_string(),
            version: Some(value.version.to_string()),
        }
    }
}
