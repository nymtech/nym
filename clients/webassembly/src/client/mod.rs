// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::config::Config;
use client_connections::{ConnectionCommandReceiver, LaneQueueLengths, TransmissionLane};
use client_core::client::{
    cover_traffic_stream::LoopCoverTrafficStream,
    inbound_messages::{InputMessage, InputMessageReceiver, InputMessageSender},
    key_manager::KeyManager,
    mix_traffic::{BatchMixMessageSender, MixTrafficController},
    real_messages_control::{self, RealMessagesController},
    received_buffer::{
        ReceivedBufferMessage, ReceivedBufferRequestReceiver, ReceivedBufferRequestSender,
        ReceivedMessagesBufferController,
    },
    topology_control::{TopologyAccessor, TopologyRefresher, TopologyRefresherConfig},
};
use crypto::asymmetric::identity;
use futures::channel::mpsc;
use futures::StreamExt;
use gateway_client::{
    AcknowledgementReceiver, AcknowledgementSender, GatewayClient, MixnetMessageReceiver,
    MixnetMessageSender,
};
use nymsphinx::addressing::clients::Recipient;
use rand::rngs::OsRng;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::console_log;

pub mod config;

#[wasm_bindgen]
pub struct NymClient {
    config: Config,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,

    // TODO: this should be stored somewhere persistently
    // received_keys: HashSet<SURBEncryptionKey>,
    /// Channel used for transforming 'raw' messages into sphinx packets and sending them
    /// through the mix network.
    input_tx: Option<InputMessageSender>,

    // callbacks
    on_message: Option<js_sys::Function>,
    on_binary_message: Option<js_sys::Function>,
    on_gateway_connect: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl NymClient {
    #[wasm_bindgen(constructor)]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            key_manager: Self::setup_key_manager(),
            on_message: None,
            on_binary_message: None,
            on_gateway_connect: None,
            input_tx: None,
        }
    }

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
        Recipient::new(
            *self.key_manager.identity_keypair().public_key(),
            *self.key_manager.encryption_keypair().public_key(),
            identity::PublicKey::from_base58_string(&self.config.gateway_endpoint.gateway_id)
                .expect("no gateway has been selected"),
        )
    }

    pub fn self_address(&self) -> String {
        self.as_mix_recipient().to_string()
    }

    // future constantly pumping loop cover traffic at some specified average rate
    // the pumped traffic goes to the MixTrafficController
    fn start_cover_traffic_stream(
        &self,
        topology_accessor: TopologyAccessor,
        mix_tx: BatchMixMessageSender,
    ) {
        console_log!("Starting loop cover traffic stream...");

        let mut stream = LoopCoverTrafficStream::new(
            self.key_manager.ack_key(),
            self.config.debug.average_ack_delay,
            self.config.debug.average_packet_delay,
            self.config.debug.loop_cover_traffic_average_delay,
            mix_tx,
            self.as_mix_recipient(),
            topology_accessor,
        );

        if let Some(size) = &self.config.debug.use_extended_packet_size {
            stream.set_custom_packet_size(size.clone().into());
        }

        stream.start();
    }

    fn start_real_traffic_controller(
        &self,
        topology_accessor: TopologyAccessor,
        ack_receiver: AcknowledgementReceiver,
        input_receiver: InputMessageReceiver,
        mix_sender: BatchMixMessageSender,
        client_connection_rx: ConnectionCommandReceiver,
        lane_queue_lengths: LaneQueueLengths,
    ) {
        let mut controller_config = real_messages_control::Config::new(
            self.key_manager.ack_key(),
            self.config.debug.ack_wait_multiplier,
            self.config.debug.ack_wait_addition,
            self.config.debug.average_ack_delay,
            self.config.debug.message_sending_average_delay,
            self.config.debug.average_packet_delay,
            self.config.debug.disable_main_poisson_packet_distribution,
            self.as_mix_recipient(),
        );

        if let Some(size) = &self.config.debug.use_extended_packet_size {
            controller_config.set_custom_packet_size(size.clone().into());
        }

        console_log!("Starting real traffic stream...");

        RealMessagesController::new(
            controller_config,
            ack_receiver,
            input_receiver,
            mix_sender,
            topology_accessor,
            lane_queue_lengths,
            client_connection_rx,
        )
        .start();
    }

    // buffer controlling all messages fetched from provider
    // required so that other components would be able to use them (say the websocket)
    fn start_received_messages_buffer_controller(
        &self,
        query_receiver: ReceivedBufferRequestReceiver,
        mixnet_receiver: MixnetMessageReceiver,
    ) {
        console_log!("Starting received messages buffer controller...");
        ReceivedMessagesBufferController::new(
            self.key_manager.encryption_keypair(),
            query_receiver,
            mixnet_receiver,
        )
        .start()
    }

    async fn start_gateway_client(
        &mut self,
        mixnet_message_sender: MixnetMessageSender,
        ack_sender: AcknowledgementSender,
    ) -> GatewayClient {
        let gateway_id = self.config.gateway_endpoint.gateway_id.clone();
        if gateway_id.is_empty() {
            panic!("The identity of the gateway is unknown - did you run `get_gateway()`?")
        }
        let gateway_owner = self.config.gateway_endpoint.gateway_owner.clone();
        if gateway_owner.is_empty() {
            panic!("The owner of the gateway is unknown - did you run `get_gateway()`?")
        }
        let gateway_address = self.config.gateway_endpoint.gateway_listener.clone();
        if gateway_address.is_empty() {
            panic!("The address of the gateway is unknown - did you run `get_gateway()`?")
        }

        let gateway_identity = identity::PublicKey::from_base58_string(gateway_id)
            .expect("provided gateway id is invalid!");

        let mut gateway_client = GatewayClient::new(
            gateway_address,
            self.key_manager.identity_keypair(),
            gateway_identity,
            gateway_owner,
            None,
            mixnet_message_sender,
            ack_sender,
            self.config.debug.gateway_response_timeout,
            None,
        );

        gateway_client.set_disabled_credentials_mode(self.config.disabled_credentials_mode);

        let shared_keys = gateway_client
            .authenticate_and_start()
            .await
            .expect("could not authenticate and start up the gateway connection");
        self.key_manager.insert_gateway_shared_key(shared_keys);

        match self.on_gateway_connect.as_ref() {
            Some(callback) => {
                callback
                    .call0(&JsValue::null())
                    .expect("on connect callback failed!");
            }
            None => console_log!("Gateway connection established - no callback specified"),
        };

        gateway_client
    }

    // future responsible for periodically polling directory server and updating
    // the current global view of topology
    async fn start_topology_refresher(&mut self, topology_accessor: TopologyAccessor) {
        let topology_refresher_config = TopologyRefresherConfig::new(
            vec![self.config.validator_api_url.clone()],
            self.config.debug.topology_refresh_rate,
            env!("CARGO_PKG_VERSION").to_string(),
        );
        let mut topology_refresher =
            TopologyRefresher::new(topology_refresher_config, topology_accessor);
        // before returning, block entire runtime to refresh the current network view so that any
        // components depending on topology would see a non-empty view
        console_log!("Obtaining initial network topology");
        topology_refresher.refresh().await;

        // TODO: a slightly more graceful termination here
        if !topology_refresher.is_topology_routable().await {
            panic!(
                "The current network topology seem to be insufficient to route any packets through\
                - check if enough nodes and a gateway are online"
            );
        }

        console_log!("Starting topology refresher...");

        // TODO: re-enable
        topology_refresher.start();
    }

    // controller for sending sphinx packets to mixnet (either real traffic or cover traffic)
    // TODO: if we want to send control messages to gateway_client, this CAN'T take the ownership
    // over it. Perhaps GatewayClient needs to be thread-shareable or have some channel for
    // requests?
    fn start_mix_traffic_controller(gateway_client: GatewayClient) -> BatchMixMessageSender {
        console_log!("Starting mix traffic controller...");
        let (mix_traffic_controller, mix_tx) = MixTrafficController::new(gateway_client);
        mix_traffic_controller.start();
        mix_tx
    }

    // TODO: this procedure is extremely overcomplicated, because it's based off native client's behaviour
    // which doesn't fully apply in this case
    fn start_reconstructed_pusher(
        &mut self,
        received_buffer_request_sender: ReceivedBufferRequestSender,
    ) {
        let on_message = self.on_message.take();
        let on_binary_message = self.on_binary_message.take();

        spawn_local(async move {
            let (reconstructed_sender, mut reconstructed_receiver) = mpsc::unbounded();

            // tell the buffer to start sending stuff to us
            received_buffer_request_sender
                .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                    reconstructed_sender,
                ))
                .expect("the buffer request failed!");

            let this = JsValue::null();

            while let Some(reconstructed) = reconstructed_receiver.next().await {
                for msg in reconstructed {
                    if let Some(ref callback_binary) = on_binary_message {
                        let arg1 = serde_wasm_bindgen::to_value(&msg.message).unwrap();
                        callback_binary
                            .call1(&this, &arg1)
                            .expect("on binary message failed!");
                    }
                    if let Some(ref callback) = on_message {
                        if msg.reply_surb.is_some() {
                            console_log!("the received message contained a reply-surb that we do not know how to handle (yet)")
                        }
                        let stringified = String::from_utf8_lossy(&msg.message).into_owned();
                        let arg1 = serde_wasm_bindgen::to_value(&stringified).unwrap();
                        callback.call1(&this, &arg1).expect("on message failed!");
                    }
                }
            }
        });
    }

    pub async fn start(mut self) -> NymClient {
        console_log!("Starting wasm client '{}'", self.config.id);
        // channels for inter-component communication
        // TODO: make the channels be internally created by the relevant components
        // rather than creating them here, so say for example the buffer controller would create the request channels
        // and would allow anyone to clone the sender channel

        // unwrapped_sphinx_sender is the transmitter of mixnet messages received from the gateway
        // unwrapped_sphinx_receiver is the receiver for said messages - used by ReceivedMessagesBuffer
        let (mixnet_messages_sender, mixnet_messages_receiver) = mpsc::unbounded();

        // used for announcing connection or disconnection of a channel for pushing re-assembled messages to
        let (received_buffer_request_sender, received_buffer_request_receiver) = mpsc::unbounded();

        // channels responsible for controlling real messages
        let (input_sender, input_receiver) = tokio::sync::mpsc::channel::<InputMessage>(1);

        // channels responsible for controlling ack messages
        let (ack_sender, ack_receiver) = mpsc::unbounded();
        let shared_topology_accessor = TopologyAccessor::new();

        // Channel that the real traffix controller can listed to for closing connections.
        // Currently unused in the wasm client.
        let (_client_connection_tx, client_connection_rx) = mpsc::unbounded();

        // the components are started in very specific order. Unless you know what you are doing,
        // do not change that.
        self.start_topology_refresher(shared_topology_accessor.clone())
            .await;
        self.start_received_messages_buffer_controller(
            received_buffer_request_receiver,
            mixnet_messages_receiver,
        );

        let gateway_client = self
            .start_gateway_client(mixnet_messages_sender, ack_sender)
            .await;

        // The sphinx_message_sender is the transmitter for any component generating sphinx packets
        // that are to be sent to the mixnet. They are used by cover traffic stream and real
        // traffic stream.
        // The MixTrafficController then sends the actual traffic
        let sphinx_message_sender = Self::start_mix_traffic_controller(gateway_client);

        // Shared queue length data. Published by the `OutQueueController` in the client, and used
        // primarily to throttle incoming connections
        let shared_lane_queue_lengths = LaneQueueLengths::new();

        self.start_real_traffic_controller(
            shared_topology_accessor.clone(),
            ack_receiver,
            input_receiver,
            sphinx_message_sender.clone(),
            client_connection_rx,
            shared_lane_queue_lengths,
        );

        if !self.config.debug.disable_loop_cover_traffic_stream {
            self.start_cover_traffic_stream(shared_topology_accessor, sphinx_message_sender);
        }

        self.start_reconstructed_pusher(received_buffer_request_sender);
        self.input_tx = Some(input_sender);

        self
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

        self.input_tx
            .as_ref()
            .expect("start method was not called before!")
            .send(input_msg)
            .await
            .expect("InputMessageReceiver has stopped receiving!");

        self
    }
}
