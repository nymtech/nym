// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tester::{NodeTesterRequest, TestMessageExt};
use crate::topology::WasmNymTopology;
use js_sys::Promise;
use node_tester_utils::TestMessage;
use nym_client_core::client::base_client::{ClientInput, ClientState};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_topology::{MixLayer, NymTopology};
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
    /// Changes the current network topology to the provided value.
    fn change_hardcoded_topology(&self, topology: WasmNymTopology) -> Promise;

    /// Returns the current network topology.
    fn current_topology(&self) -> Promise;

    /// Checks whether the provided node exists in the known network topology and if so, returns its layer.
    fn check_for_mixnode_existence(&self, mixnode_identity: String) -> Promise;

    /// Creates a `NodeTesterRequest` with a variant of `this` topology where the target node is the only one on its layer.
    fn mix_test_request(&self, test_id: u64, mixnode_identity: String) -> Promise;
}

impl WasmTopologyExt for Arc<ClientState> {
    fn change_hardcoded_topology(&self, topology: WasmNymTopology) -> Promise {
        let this = Arc::clone(self);
        future_to_promise(async move {
            let nym_topology: NymTopology = topology.into();
            console_log!("changing topology to {nym_topology:?}");
            this.topology_accessor
                .manually_change_topology(nym_topology)
                .await;
            Ok(JsValue::null())
        })
    }

    fn current_topology(&self) -> Promise {
        let this = Arc::clone(self);
        future_to_promise(async move {
            match this.topology_accessor.current_topology().await {
                Some(topology) => Ok(JsValue::from(WasmNymTopology::from(topology))),
                None => Err(simple_js_error("Network topology is currently unavailable")),
            }
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

    fn mix_test_request(&self, test_id: u64, mixnode_identity: String) -> Promise {
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

            let test_msg = TestMessage::new_mix(mix, TestMessageExt::new(test_id));

            let mut updated = current_topology.clone();
            updated.set_mixes_in_layer(mix.layer.into(), vec![mix.to_owned()]);

            Ok(JsValue::from(NodeTesterRequest {
                test_msg,
                testable_topology: updated,
            }))
        })
    }
}
