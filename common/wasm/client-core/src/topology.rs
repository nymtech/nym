// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_topology::wasm_helpers::SerializableTopologyError;
use nym_validator_client::client::IdentityKeyRef;

pub use nym_topology::wasm_helpers::{WasmFriendlyNymTopology, WasmFriendlyRoutingNode};
pub use nym_topology::{Role, RoutingNode};

// redeclare this as a type alias for easy of use
pub type WasmTopologyError = SerializableTopologyError;

// helper trait to define extra functionality on the external type that we used to have here before
pub trait SerializableTopologyExt {
    // fn print(&self);

    fn ensure_contains_gateway_id(&self, gateway_id: IdentityKeyRef) -> bool;
}

impl SerializableTopologyExt for WasmFriendlyNymTopology {
    fn ensure_contains_gateway_id(&self, gateway_id: IdentityKeyRef) -> bool {
        self.node_details
            .values()
            .any(|node| node.identity_key == gateway_id)
    }
}
