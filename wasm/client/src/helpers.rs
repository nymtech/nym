// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use js_sys::Promise;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use wasm_client_core::client::base_client::{ClientInput, ClientState};
use wasm_client_core::client::inbound_messages::InputMessage;
use wasm_client_core::error::WasmCoreError;
use wasm_client_core::topology::WasmNymTopology;
use wasm_client_core::{MixLayer, NymTopology};
use wasm_utils::console_log;
use wasm_utils::error::simple_js_error;

#[cfg(feature = "node-tester")]
use nym_node_tester_wasm::types::{NodeTestMessage, WasmTestMessageExt};

#[cfg(feature = "node-tester")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "node-tester")]
pub(crate) const DEFAULT_TEST_PACKETS: u32 = 20;

#[cfg(feature = "node-tester")]
#[wasm_bindgen]
pub struct NymClientTestRequest {
    // serialized NodeTestMessage
    pub(crate) test_msgs: Vec<Vec<u8>>,

    // specially constructed network topology that only contains the target
    // node on the tested layer
    pub(crate) testable_topology: NymTopology,
}

#[cfg(feature = "node-tester")]
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
}

#[cfg(feature = "node-tester")]
pub(crate) trait WasmTopologyTestExt {
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
                None => Err(WasmCoreError::UnavailableNetworkTopology.into()),
            }
        })
    }

    /// Checks whether the target mixnode exists in the known network topology and returns its layer.
    fn check_for_mixnode_existence(&self, mixnode_identity: String) -> Promise {
        let this = Arc::clone(self);
        future_to_promise(async move {
            let Some(current_topology) = this.topology_accessor.current_topology().await else {
                return Err(WasmCoreError::UnavailableNetworkTopology.into())
            };

            match current_topology.find_mix_by_identity(&mixnode_identity) {
                None => Err(WasmCoreError::NonExistentMixnode { mixnode_identity }.into()),
                Some(node) => Ok(JsValue::from(MixLayer::from(node.layer))),
            }
        })
    }
}

#[cfg(feature = "node-tester")]
impl WasmTopologyTestExt for Arc<ClientState> {
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
                return Err(WasmCoreError::UnavailableNetworkTopology.into())
            };

            let Some(mix) = current_topology.find_mix_by_identity(&mixnode_identity) else {
                return Err(WasmCoreError::NonExistentMixnode { mixnode_identity }.into());
            };

            let ext = WasmTestMessageExt::new(test_id);
            let test_msgs = NodeTestMessage::mix_plaintexts(mix, num_test_packets, ext)
                .map_err(crate::error::WasmClientError::from)?;

            let mut updated = current_topology.clone();
            updated.set_mixes_in_layer(mix.layer.into(), vec![mix.to_owned()]);

            Ok(JsValue::from(NymClientTestRequest {
                test_msgs,
                testable_topology: updated,
            }))
        })
    }
}
