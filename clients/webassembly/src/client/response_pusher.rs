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
    on_message: Option<js_sys::Function>,
    on_binary_message: Option<js_sys::Function>,
}

impl ResponsePusher {
    pub(crate) fn new(
        client_output: ClientOutput,
        on_message: Option<js_sys::Function>,
        on_binary_message: Option<js_sys::Function>,
    ) -> Self {
        if on_message.is_none() && on_binary_message.is_none() {
            // exercise for the reader : )
            panic!("neither 'on_message' nor 'on_binary_message' was set!")
        }

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
            on_binary_message,
        }
    }

    pub(crate) fn start(mut self) {
        spawn_local(async move {
            let this = JsValue::null();

            while let Some(reconstructed) = self.reconstructed_receiver.next().await {
                for msg in reconstructed {
                    if let Some(ref callback_binary) = self.on_binary_message {
                        let arg1 = serde_wasm_bindgen::to_value(&msg.message).unwrap();
                        callback_binary
                            .call1(&this, &arg1)
                            .expect("on binary message failed!");
                    }
                    if let Some(ref callback) = self.on_message {
                        if msg.reply_surb.is_some() {
                            console_log!("the received message contained a reply-surb that we do not know how to handle (yet)")
                        }
                        let stringified = String::from_utf8_lossy(&msg.message).into_owned();
                        let arg1 = serde_wasm_bindgen::to_value(&stringified).unwrap();
                        callback.call1(&this, &arg1).expect("on message failed!");
                    }
                }
            }
        })
    }
}
