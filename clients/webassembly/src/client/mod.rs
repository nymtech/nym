// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::config::Config;
use crate::client::response_pusher::ResponsePusher;
use client_connections::TransmissionLane;
use client_core::client::base_client::{BaseClientBuilder, ClientInput, ClientOutput};
use client_core::client::{inbound_messages::InputMessage, key_manager::KeyManager};
use crypto::asymmetric::identity;
use nymsphinx::addressing::clients::Recipient;
use rand::rngs::OsRng;
use task::ShutdownNotifier;
use wasm_bindgen::prelude::*;
use wasm_utils::{console_error, console_log};

pub mod config;
mod response_pusher;

#[wasm_bindgen]
pub struct NymClient {
    config: Config,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    // due to disgusting workaround I had to wrap the key_manager in an Option
    // so that the interface wouldn't change (i.e. both `start` and `new` would still return a `NymClient`)
    key_manager: Option<KeyManager>,
    self_address: Option<String>,

    // TODO: this should be stored somewhere persistently
    // received_keys: HashSet<SURBEncryptionKey>,
    /// Channel used for transforming 'raw' messages into sphinx packets and sending them
    /// through the mix network.
    client_input: Option<ClientInput>,

    // callbacks
    on_message: Option<js_sys::Function>,
    on_binary_message: Option<js_sys::Function>,
    on_gateway_connect: Option<js_sys::Function>,

    // even though we don't use graceful shutdowns, other components rely on existence of this struct
    // and if it's dropped, everything will start going offline
    _shutdown: Option<ShutdownNotifier>,
}

#[wasm_bindgen]
impl NymClient {
    #[wasm_bindgen(constructor)]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            key_manager: Some(Self::setup_key_manager()),
            on_message: None,
            on_binary_message: None,
            on_gateway_connect: None,
            client_input: None,
            self_address: None,
            _shutdown: None,
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

    pub fn set_on_message(&mut self, on_message: js_sys::Function) {
        self.on_message = Some(on_message);
    }

    pub fn set_on_binary_message(&mut self, on_binary_message: js_sys::Function) {
        self.on_binary_message = Some(on_binary_message);
    }

    pub fn set_on_gateway_connect(&mut self, on_connect: js_sys::Function) {
        self.on_gateway_connect = Some(on_connect)
    }

    fn as_mix_recipient(&self) -> Recipient {
        // another disgusting (and hopefully temporary) workaround
        let key_manager_ref = self
            .key_manager
            .as_ref()
            .expect("attempting to call 'as_mix_recipient' after 'start'");

        Recipient::new(
            *key_manager_ref.identity_keypair().public_key(),
            *key_manager_ref.encryption_keypair().public_key(),
            identity::PublicKey::from_base58_string(&self.config.gateway_endpoint.gateway_id)
                .expect("no gateway has been selected"),
        )
    }

    pub fn self_address(&self) -> String {
        if let Some(address) = &self.self_address {
            address.clone()
        } else {
            self.as_mix_recipient().to_string()
        }
    }

    // Right now it's impossible to have async exported functions to take `&mut self` rather than mut self
    // TODO: try Rc<RefCell<Self>> approach?
    pub async fn send_message(self, message: String, recipient: String) -> Self {
        console_log!("Sending {} to {}", message, recipient);

        let message_bytes = message.into_bytes();
        self.send_binary_message(message_bytes, recipient).await
    }

    pub async fn send_binary_message(self, message: Vec<u8>, recipient: String) -> Self {
        console_log!("Sending {} bytes to {}", message.len(), recipient);

        let recipient = Recipient::try_from_base58_string(recipient).unwrap();
        let lane = TransmissionLane::General;

        let input_msg = InputMessage::new_fresh(recipient, message, false, lane);

        self.client_input
            .as_ref()
            .expect("start method was not called before!")
            .input_sender
            .send(input_msg)
            .await
            .expect("InputMessageReceiver has stopped receiving!");

        self
    }

    fn start_reconstructed_pusher(
        client_output: ClientOutput,
        on_message: Option<js_sys::Function>,
        on_binary_message: Option<js_sys::Function>,
    ) {
        ResponsePusher::new(client_output, on_message, on_binary_message).start()
    }

    pub async fn start(mut self) -> NymClient {
        console_log!("Starting the wasm client");

        let base_builder = BaseClientBuilder::new(
            &self.config.gateway_endpoint,
            &self.config.debug,
            self.key_manager.take().unwrap(),
            None,
            true,
            vec![self.config.validator_api_url.clone()],
        );

        self.self_address = Some(base_builder.as_mix_recipient().to_string());
        let mut started_client = match base_builder.start_base().await {
            Ok(base_client) => base_client,
            Err(err) => {
                console_error!("failed to start base client components - {}", err);
                // proper error handling is left here as an exercise for the reader (hi Mark : ))
                panic!("failed to start base client components - {err}")
            }
        };
        match self.on_gateway_connect.as_ref() {
            Some(callback) => {
                callback
                    .call0(&JsValue::null())
                    .expect("on connect callback failed!");
            }
            None => console_log!("Gateway connection established - no callback specified"),
        };

        // those should be moved to a completely different struct, but I don't want to break compatibility for now
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        let on_message = self.on_message.take();
        let on_binary_message = self.on_binary_message.take();
        Self::start_reconstructed_pusher(client_output, on_message, on_binary_message);
        self.client_input = Some(client_input);
        self._shutdown = Some(started_client.shutdown_notifier);

        self
    }
}
