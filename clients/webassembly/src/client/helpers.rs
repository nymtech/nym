// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use client_core::client::base_client::ClientInput;
use client_core::client::inbound_messages::InputMessage;
use js_sys::Promise;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;

use nym_client_core::client::base_client::{ClientInput, ClientState};
use nym_client_core::client::inbound_messages::InputMessage;
use wasm_utils::console_log;

use crate::topology::WasmNymTopology;

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
                Err(_) => {
                    let js_error =
                        js_sys::Error::new("InputMessageReceiver has stopped receiving!");
                    Err(JsValue::from(js_error))
                }
            }
        })
    }
}

pub(crate) trait WasmTopologyExt {
    fn change_hardcoded_topology(&self, topology: WasmNymTopology) -> Promise;

    fn check_for_mixnode_existence(&self, mixnode_identity: String) -> Promise;
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
                let js_error =
                    js_sys::Error::new("Network topology is currently unavailable");
                return Err(JsValue::from(js_error));
            };

            for (&layer, mixes) in current_topology.mixes() {
                for mix in mixes {
                    if mix.identity_key.to_base58_string() == mixnode_identity {
                        return Ok(JsValue::from(layer));
                    }
                }
            }

            let js_error = js_sys::Error::new(&format!(
                "The current network topology does not contain mixnode {mixnode_identity}"
            ));
            Err(JsValue::from(js_error))
        })
    }
}
