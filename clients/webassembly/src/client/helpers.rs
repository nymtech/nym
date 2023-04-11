// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tester::NodeTesterRequest;
use crate::topology::WasmNymTopology;
use js_sys::Promise;
use nym_client_core::client::base_client::{ClientInput, ClientState};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_topology::MixLayer;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_log, simple_js_error};

// defining helper trait as we could directly call the method on the wrapper
pub(crate) trait InputSender {
    fn send_message(&self, message: InputMessage) -> Promise;
}

impl InputSender for Arc<ClientInput> {
    fn send_message(&self, message: InputMessage) -> Promise {
        let this = Arc::clone(self);
        future_to_promise(async move {
            match this.input_sender.send(message).await {
                Ok(_) => Ok(JsValue::null()),
                Err(_) => Err(simple_js_error(
                    "InputMessageReceiver has stopped receiving!",
                )),
            }
        })
    }
}

pub(crate) trait WasmTopologyExt {
    fn change_hardcoded_topology(&self, topology: WasmNymTopology) -> Promise;

    fn check_for_mixnode_existence(&self, mixnode_identity: String) -> Promise;

    /// Gets a variant of `this` topology where the target node is the only one on its layer
    fn reduced_layer_topology(&self, mixnode_identity: String) -> Promise;
}

impl WasmTopologyExt for Arc<ClientState> {
    fn change_hardcoded_topology(&self, topology: WasmNymTopology) -> Promise {
        let this = Arc::clone(self);
        future_to_promise(async move {
            this.topology_accessor
                .manually_change_topology(topology.into())
                .await;
            Ok(JsValue::null())
        })
    }

    /// Checks whether the target mixnode exists in the known network topology and returns its layer.
    fn check_for_mixnode_existence(&self, mixnode_identity: String) -> Promise {
        let this = Arc::clone(self);
        future_to_promise(async move {
            let Some(current_topology) = this.topology_accessor.current_topology().await else {
                return Err(simple_js_error("Network topology is currently unavailable"))
            };

            match current_topology.find_mix_by_identity(&mixnode_identity) {
                None => Err(simple_js_error(format!(
                    "The current network topology does not contain mixnode {mixnode_identity}"
                ))),
                Some(node) => Ok(JsValue::from(MixLayer::from(node.layer))),
            }
        })
    }

    fn reduced_layer_topology(&self, mixnode_identity: String) -> Promise {
        let this = Arc::clone(self);
        future_to_promise(async move {
            let Some(current_topology) = this.topology_accessor.current_topology().await else {
                return Err(simple_js_error("Network topology is currently unavailable"))
            };

            let Some(mix) = current_topology.find_mix_by_identity(&mixnode_identity) else {
                    return Err(simple_js_error(format!(
                    "The current network topology does not contain mixnode {mixnode_identity}"
                )))
            };

            let mut updated = current_topology.clone();
            updated.set_mixes_in_layer(mix.layer.into(), vec![mix.to_owned()]);

            Ok(JsValue::from(NodeTesterRequest {
                id: todo!(),
                testable_topology: updated,
            }))
        })
    }
}
