// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use js_sys::Promise;
use nym_wasm_client_core::client::base_client::{ClientInput, ClientState};
use nym_wasm_client_core::client::inbound_messages::InputMessage;
use nym_wasm_client_core::error::WasmCoreError;
use nym_wasm_client_core::topology::WasmFriendlyNymTopology;
use nym_wasm_client_core::NymTopology;
use nym_wasm_utils::error::simple_js_error;
use nym_wasm_utils::{check_promise_result, console_log};
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;

// defining helper trait as we could directly call the method on the wrapper
pub(crate) trait InputSender {
    fn send_message(&self, message: InputMessage) -> Promise;

    #[allow(dead_code)]
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
    fn change_hardcoded_topology(&self, topology: WasmFriendlyNymTopology) -> Promise;

    /// Returns the current network topology.
    fn current_topology(&self) -> Promise;
}

impl WasmTopologyExt for Arc<ClientState> {
    fn change_hardcoded_topology(&self, topology: WasmFriendlyNymTopology) -> Promise {
        let nym_topology: NymTopology = check_promise_result!(topology.try_into());

        let this = Arc::clone(self);
        future_to_promise(async move {
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
            match this.topology_accessor.current_route_provider().await {
                Some(route_provider) => Ok(serde_wasm_bindgen::to_value(
                    &WasmFriendlyNymTopology::from(route_provider.topology.clone()),
                )
                .expect("WasmFriendlyNymTopology failed serialization")),
                None => Err(WasmCoreError::UnavailableNetworkTopology.into()),
            }
        })
    }
}
