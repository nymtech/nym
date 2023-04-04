// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_topology::mix::Layer;
use nym_topology::{gateway, mix, MixLayer, NymTopology};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use validator_client::client::MixId;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

pub type PlaceholderError = String;
//
// // TODO: perhaps move elsewhere
// #[derive(Clone)]
// pub struct Base58String(String);
//
// impl serde::Serialize for Base58String {
//     #[inline(always)]
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         self.0.serialize(serializer)
//     }
// }
//
// impl<'de> serde::Deserialize<'de> for Base58String {
//     #[inline(always)]
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         let s = <String>::deserialize(deserializer)?;
//         // make sure it's a valid bs58 string
//         let _bytes = bs58::decode(&s)
//             .into_vec()
//             .map_err(serde::de::Error::custom)?;
//         Ok(Base58String(s))
//     }
// }

// impl Deref for Base58String {
//
// }

#[wasm_bindgen]
pub struct WasmNymTopology {
    mixnodes: HashMap<Layer, Vec<WasmMixNode>>,
    gateways: Vec<WasmGateway>,
}

#[wasm_bindgen]
impl WasmNymTopology {
    #[wasm_bindgen(constructor)]
    pub fn new(
        // expected: HashMap<Layer, Vec<WasmMixNode>>,
        mixnodes: JsValue,
        // expected: Vec<WasmGateway>
        gateways: JsValue,
    ) -> Result<WasmNymTopology, PlaceholderError> {
        let mixnodes: HashMap<Layer, Vec<WasmMixNode>> =
            serde_wasm_bindgen::from_value(mixnodes).expect("TODO");

        let gateways: Vec<WasmGateway> = serde_wasm_bindgen::from_value(gateways).expect("TODO");

        Ok(WasmNymTopology { mixnodes, gateways })
    }
}

impl TryFrom<WasmNymTopology> for NymTopology {
    type Error = ();

    fn try_from(value: WasmNymTopology) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct WasmMixNode {
    pub mix_id: MixId,
    #[wasm_bindgen(getter_with_clone)]
    pub owner: String,
    #[wasm_bindgen(getter_with_clone)]
    pub host: String,
    #[wasm_bindgen(getter_with_clone)]
    pub identity_key: String,
    #[wasm_bindgen(getter_with_clone)]
    pub sphinx_key: String,
    pub layer: MixLayer,
    #[wasm_bindgen(getter_with_clone)]
    pub version: String,
}

impl TryFrom<WasmMixNode> for mix::Node {
    type Error = ();

    fn try_from(value: WasmMixNode) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct WasmGateway {
    #[wasm_bindgen(getter_with_clone)]
    pub owner: String,
    #[wasm_bindgen(getter_with_clone)]
    pub host: String,
    pub clients_port: u16,
    #[wasm_bindgen(getter_with_clone)]
    pub identity_key: String,
    #[wasm_bindgen(getter_with_clone)]
    pub sphinx_key: String,
    #[wasm_bindgen(getter_with_clone)]
    pub version: String,
}

impl TryFrom<WasmGateway> for gateway::Node {
    type Error = ();

    fn try_from(value: WasmGateway) -> Result<Self, Self::Error> {
        todo!()
    }
}
