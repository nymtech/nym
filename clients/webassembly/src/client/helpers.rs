// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use js_sys::Promise;
use nym_client_core::client::base_client::ClientInput;
use nym_client_core::client::inbound_messages::InputMessage;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;

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
