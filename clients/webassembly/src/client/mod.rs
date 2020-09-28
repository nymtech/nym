// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::built_info;
use crypto::asymmetric::{encryption, identity};
use directory_client::DirectoryClient;
use futures::channel::mpsc;
use gateway_client::GatewayClient;
use js_sys::Promise;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::preparer::MessagePreparer;
use rand::rngs::OsRng;
use received_processor::ReceivedMessagesProcessor;
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration;
use topology::{gateway, NymTopology};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, spawn_local};
use wasm_utils::{console_log, console_warn};

pub(crate) mod received_processor;

const DEFAULT_RNG: OsRng = OsRng;

const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_AVERAGE_ACK_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_millis(1_500);

#[wasm_bindgen]
pub struct NymClient {
    directory_server: String,

    // TODO: technically this doesn't need to be an Arc since wasm is run on a single thread
    // however, once we eventually combine this code with the native-client's, it will make things
    // easier.
    identity: Arc<identity::KeyPair>,
    encryption_keys: Arc<encryption::KeyPair>,
    ack_key: Arc<AckKey>,

    message_preparer: Option<MessagePreparer<OsRng>>,
    // message_receiver: MessageReceiver,

    // TODO: this should be stored somewhere persistently
    // received_keys: HashSet<SURBEncryptionKey>,
    topology: Option<NymTopology>,
    gateway_client: Option<GatewayClient>,

    // callbacks
    on_message: Option<js_sys::Function>,
    on_gateway_connect: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl NymClient {
    #[wasm_bindgen(constructor)]
    pub fn new(directory_server: String) -> Self {
        // for time being generate new keys each time...
        let identity = identity::KeyPair::new_with_rng(&mut DEFAULT_RNG);
        let encryption_keys = encryption::KeyPair::new_with_rng(&mut DEFAULT_RNG);
        let ack_key = AckKey::new(&mut DEFAULT_RNG);

        Self {
            identity: Arc::new(identity),
            encryption_keys: Arc::new(encryption_keys),
            ack_key: Arc::new(ack_key),
            directory_server,
            message_preparer: None,
            // received_keys: Default::default(),
            topology: None,
            gateway_client: None,

            on_message: None,
            on_gateway_connect: None,
        }
    }

    pub fn set_on_message(&mut self, on_message: js_sys::Function) {
        self.on_message = Some(on_message);
    }

    pub fn set_on_gateway_connect(&mut self, on_connect: js_sys::Function) {
        console_log!("setting on connect...");
        self.on_gateway_connect = Some(on_connect)
    }

    fn self_recipient(&self) -> Recipient {
        Recipient::new(
            self.identity.public_key().clone(),
            self.encryption_keys.public_key().clone(),
            self.gateway_client
                .as_ref()
                .expect("gateway connection was not established!")
                .identity(),
        )
    }

    pub fn self_address(&self) -> String {
        self.self_recipient().to_string()
    }

    // Right now it's impossible to have async exported functions to take `&self` rather than self
    pub async fn initial_setup(self) -> Self {
        let mut client = self.get_and_update_topology().await;
        let gateway = client.choose_gateway();

        let (mixnet_messages_sender, mixnet_messages_receiver) = mpsc::unbounded();
        let (ack_sender, ack_receiver) = mpsc::unbounded();

        let mut gateway_client = GatewayClient::new(
            gateway.client_listener.clone(),
            Arc::clone(&client.identity),
            gateway.identity_key,
            None,
            mixnet_messages_sender,
            ack_sender,
            DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
        );

        gateway_client
            .authenticate_and_start()
            .await
            .expect("could not authenticate and start up the gateway connection");

        client.gateway_client = Some(gateway_client);
        match client.on_gateway_connect.as_ref() {
            Some(callback) => {
                callback
                    .call0(&JsValue::null())
                    .expect("on connect callback failed!");
            }
            None => console_log!("Gateway connection established - no callback specified"),
        };

        let message_preparer = MessagePreparer::new(
            DEFAULT_RNG,
            client.self_recipient(),
            DEFAULT_AVERAGE_PACKET_DELAY,
            DEFAULT_AVERAGE_ACK_DELAY,
        );

        let received_processor = ReceivedMessagesProcessor::new(
            Arc::clone(&client.encryption_keys),
            Arc::clone(&client.ack_key),
        );

        client.message_preparer = Some(message_preparer);

        spawn_local(received_processor.start_processing(
            mixnet_messages_receiver,
            ack_receiver,
            client.on_message.take().expect("on_message was not set!"),
        ));

        client
    }

    // Right now it's impossible to have async exported functions to take `&mut self` rather than mut self
    // TODO: try Rc<RefCell<Self>> approach?
    pub async fn send_message(mut self, message: String, recipient: String) -> Self {
        console_log!("Sending {} to {}", message, recipient);

        let message_bytes = message.into_bytes();
        let recipient = Recipient::try_from_base58_string(recipient).unwrap();

        let topology = self
            .topology
            .as_ref()
            .expect("did not obtain topology before");

        let message_preparer = self.message_preparer.as_mut().unwrap();

        let (split_message, _reply_keys) = message_preparer
            .prepare_and_split_message(message_bytes, false, topology)
            .expect("failed to split the message");

        let mut socket_messages = Vec::with_capacity(split_message.len());
        for message_chunk in split_message {
            // don't bother with acks etc. for time being
            let prepared_fragment = message_preparer
                .prepare_chunk_for_sending(message_chunk, topology, &self.ack_key, &recipient)
                .unwrap();

            console_warn!("packet is going to have round trip time of {:?}, but we're not going to do anything for acks anyway ", prepared_fragment.total_delay);
            socket_messages.push((
                prepared_fragment.first_hop_address,
                prepared_fragment.sphinx_packet,
            ));
        }
        self.gateway_client
            .as_mut()
            .unwrap()
            .batch_send_sphinx_packets(socket_messages)
            .await
            .unwrap();
        self
    }

    pub(crate) fn choose_gateway(&self) -> &gateway::Node {
        let topology = self
            .topology
            .as_ref()
            .expect("did not obtain topology before");

        // choose the first one available
        assert!(!topology.gateways().is_empty());
        topology.gateways().first().unwrap()
    }

    // Right now it's impossible to have async exported functions to take `&mut self` rather than mut self
    // self: Rc<Self>
    // or this: Rc<RefCell<Self>>
    pub async fn get_and_update_topology(mut self) -> Self {
        let new_topology = self.get_nym_topology().await;
        self.update_topology(new_topology);
        self
    }

    pub(crate) fn update_topology(&mut self, topology: NymTopology) {
        self.topology = Some(topology)
    }

    pub fn get_full_topology_json(&self) -> Promise {
        let directory_client_config = directory_client::Config::new(self.directory_server.clone());
        let directory_client = directory_client::Client::new(directory_client_config);
        future_to_promise(async move {
            let topology = &directory_client.get_topology().await.unwrap();
            Ok(JsValue::from_serde(&topology).unwrap())
        })
    }

    pub(crate) async fn get_nym_topology(&self) -> NymTopology {
        let directory_client_config = directory_client::Config::new(self.directory_server.clone());
        let directory_client = directory_client::Client::new(directory_client_config);

        match directory_client.get_topology().await {
            Err(err) => panic!(err),
            Ok(topology) => {
                let nym_topology: NymTopology = topology
                    .try_into()
                    .ok()
                    .expect("this is not a NYM topology!");
                let version = built_info::PKG_VERSION;
                nym_topology.filter_system_version(&version)
            }
        }
    }
}
