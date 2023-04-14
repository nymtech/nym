// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tester::helpers::WasmTestMessageExt;
use crate::tester::{NodeTestMessage, DEFAULT_TEST_PACKETS};
use crate::topology::WasmNymTopology;
use js_sys::Promise;
use nym_client_core::client::base_client::{ClientInput, ClientState};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_topology::{MixLayer, NymTopology};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_log, js_error, simple_js_error};

#[wasm_bindgen]
pub struct NymClientTestRequest {
    // serialized NodeTestMessage
    pub(crate) test_msgs: Vec<Vec<u8>>,

    // specially constructed network topology that only contains the target
    // node on the tested layer
    pub(crate) testable_topology: NymTopology,
}

#[wasm_bindgen]
impl NymClientTestRequest {
    pub fn injectable_topology(&self) -> WasmNymTopology {
        self.testable_topology.clone().into()
    }
}

// defining helper trait as we could directly call the method on the wrapper
pub(crate) trait InputSender {
    fn send_message(&self, message: InputMessage) -> Promise;

    fn send_messages(&self, messages: Vec<InputMessage>) -> Promise;
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

    fn send_messages(&self, messages: Vec<InputMessage>) -> Promise {
        let this = Arc::clone(self);
        future_to_promise(async move {
            for message in messages {
                if this.input_sender.send(message).await.is_err() {
                    return Err(simple_js_error(
                        "InputMessageReceiver has stopped receiving!",
                    ));
                }
            }
            Ok(JsValue::null())
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

    /// Creates a `NymClientTestRequest` with a variant of `this` topology where the target node is the only one on its layer.
    fn mix_test_request(
        &self,
        test_id: u32,
        mixnode_identity: String,
        num_test_packets: Option<u32>,
    ) -> Promise;
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

    fn mix_test_request(
        &self,
        test_id: u32,
        mixnode_identity: String,
        num_test_packets: Option<u32>,
    ) -> Promise {
        let num_test_packets = num_test_packets.unwrap_or(DEFAULT_TEST_PACKETS);

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

            let mut test_msgs = Vec::with_capacity(num_test_packets as usize);
            for i in 1..=num_test_packets {
                let msg = NodeTestMessage::new_mix(
                    mix,
                    i,
                    num_test_packets,
                    WasmTestMessageExt::new(test_id),
                );
                let serialized = match msg.as_bytes() {
                    Ok(bytes) => bytes,
                    Err(err) => return Err(js_error!("failed to serialize test message: {err}")),
                };
                test_msgs.push(serialized);
            }

            let mut updated = current_topology.clone();
            updated.set_mixes_in_layer(mix.layer.into(), vec![mix.to_owned()]);

            Ok(JsValue::from(NymClientTestRequest {
                test_msgs,
                testable_topology: updated,
            }))
        })
    }
}
