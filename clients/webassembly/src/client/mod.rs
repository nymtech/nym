// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::config::Config;
use crate::client::response_pusher::ResponsePusher;
use client_connections::TransmissionLane;
use client_core::client::base_client::{BaseClientBuilder, ClientInput, ClientOutput};
use client_core::client::replies::reply_storage::browser_backend;
use client_core::client::{inbound_messages::InputMessage, key_manager::KeyManager};
use gateway_client::bandwidth::BandwidthController;
use js_sys::Promise;
use nymsphinx::addressing::clients::Recipient;
use rand::rngs::OsRng;
use std::sync::Arc;
use task::ShutdownNotifier;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_error, console_log};

pub mod config;
mod response_pusher;

#[wasm_bindgen]
pub struct NymClient {
    #[wasm_bindgen(getter_with_clone)]
    pub self_address: String,
    client_input: Arc<ClientInput>,

    // even though we don't use graceful shutdowns, other components rely on existence of this struct
    // and if it's dropped, everything will start going offline
    _shutdown: ShutdownNotifier,
}

#[wasm_bindgen]
pub struct NymClientBuilder {
    config: Config,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,

    reply_surb_storage_backend: browser_backend::Backend,

    on_message: js_sys::Function,

    // unimplemented:
    bandwidth_controller: Option<BandwidthController>,
    disabled_credentials: bool,
}

#[wasm_bindgen]
impl NymClientBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(config: Config, on_message: js_sys::Function) -> Self {
        //, key_manager: Option<KeyManager>) {
        NymClientBuilder {
            reply_surb_storage_backend: Self::setup_reply_surb_storage_backend(&config),
            config,
            key_manager: Self::setup_key_manager(),
            on_message,
            bandwidth_controller: None,
            disabled_credentials: true,
        }
    }

    // TODO: once we make keys persistent, we'll require some kind of `init` method to generate
    // a prior shared keypair between the client and the gateway

    // perhaps this should be public?
    fn setup_key_manager() -> KeyManager {
        let mut rng = OsRng;
        // for time being generate new keys each time...
        console_log!("generated new set of keys");
        KeyManager::new(&mut rng)
    }

    // don't get too excited about the name, under the hood it's just a big fat placeholder
    // with no persistence
    fn setup_reply_surb_storage_backend(config: &Config) -> browser_backend::Backend {
        browser_backend::Backend::new(
            config.debug.minimum_reply_surb_storage_threshold,
            config.debug.maximum_reply_surb_storage_threshold,
        )
    }

    fn start_reconstructed_pusher(client_output: ClientOutput, on_message: js_sys::Function) {
        ResponsePusher::new(client_output, on_message).start()
    }

    pub async fn start_client(self) -> Promise {
        future_to_promise(async move {
            console_log!("Starting the wasm client");

            let base_builder = BaseClientBuilder::new(
                &self.config.gateway_endpoint,
                &self.config.debug,
                self.key_manager,
                self.bandwidth_controller,
                self.reply_surb_storage_backend,
                self.disabled_credentials,
                vec![self.config.validator_api_url.clone()],
            );

            let self_address = base_builder.as_mix_recipient().to_string();
            let mut started_client = match base_builder.start_base().await {
                Ok(base_client) => base_client,
                Err(err) => {
                    let error_msg = format!("failed to start the base client components - {err}");
                    console_error!("{}", error_msg);
                    let js_error = js_sys::Error::new(&error_msg);
                    return Err(JsValue::from(js_error));
                }
            };

            let client_input = started_client.client_input.register_producer();
            let client_output = started_client.client_output.register_consumer();

            Self::start_reconstructed_pusher(client_output, self.on_message);

            Ok(JsValue::from(NymClient {
                self_address,
                client_input: Arc::new(client_input),
                _shutdown: started_client.shutdown_notifier,
            }))
        })
    }
}

#[wasm_bindgen]
impl NymClient {
    pub fn send_message(&self, message: Vec<u8>, recipient: String) -> Promise {
        console_log!("Sending {} bytes to {}", message.len(), recipient);

        let recipient = Recipient::try_from_base58_string(recipient).unwrap();
        let lane = TransmissionLane::General;

        let input_msg = InputMessage::new_regular(recipient, message, lane);

        let input = Arc::clone(&self.client_input);

        future_to_promise(async move {
            match input.input_sender.send(input_msg).await {
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
