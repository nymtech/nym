// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_topology::SerializableTopologyError;
use nym_validator_client::client::IdentityKeyRef;
use wasm_utils::console_log;

pub use nym_topology::{
    gateway, mix, SerializableGateway, SerializableMixNode, SerializableNymTopology,
};

// redeclare this as a type alias for easy of use
pub type WasmTopologyError = SerializableTopologyError;

// helper trait to define extra functionality on the external type that we used to have here before
pub trait SerializableTopologyExt {
    fn print(&self);

    fn ensure_contains_gateway_id(&self, gateway_id: IdentityKeyRef) -> bool;
}

impl SerializableTopologyExt for SerializableNymTopology {
    fn print(&self) {
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

    fn ensure_contains_gateway_id(&self, gateway_id: IdentityKeyRef) -> bool {
        self.gateways.iter().any(|g| g.identity_key == gateway_id)
    }
}
