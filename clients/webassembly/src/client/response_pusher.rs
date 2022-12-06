// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use client_core::client::base_client::ClientOutput;
use client_core::client::received_buffer::{ReceivedBufferMessage, ReconstructedMessagesReceiver};
use futures::channel::mpsc;
use futures::StreamExt;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::console_log;

pub(crate) struct ResponsePusher {
    reconstructed_receiver: ReconstructedMessagesReceiver,
    on_message: js_sys::Function,
}

impl ResponsePusher {
    pub(crate) fn new(client_output: ClientOutput, on_message: js_sys::Function) -> Self {
        // register our output
        let (reconstructed_sender, reconstructed_receiver) = mpsc::unbounded();

        // tell the buffer to start sending stuff to us
        client_output
            .received_buffer_request_sender
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                reconstructed_sender,
            ))
            .expect("the buffer request failed!");

        ResponsePusher {
            reconstructed_receiver,
            on_message,
        }
    }

    pub(crate) fn start(mut self) {
        spawn_local(async move {
            let this = JsValue::null();

            while let Some(reconstructed) = self.reconstructed_receiver.next().await {
                for msg in reconstructed {
                    if msg.sender_tag.is_some() {
                        console_log!("the received message contained a sender tag (meaning we also got some surbs!), but we do not know how to handle that (yet)")
                    }
                    let arg1 = serde_wasm_bindgen::to_value(&msg.message).unwrap();
                    self.on_message
                        .call1(&this, &arg1)
                        .expect("on binary message failed!");
                }
            }
        })
    }
}
