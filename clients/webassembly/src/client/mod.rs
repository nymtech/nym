// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use client_core::client::cover_traffic_stream::LoopCoverTrafficStream;
use client_core::client::inbound_messages::InputMessage;
use client_core::client::inbound_messages::InputMessageReceiver;
use client_core::client::key_manager::KeyManager;
use client_core::client::mix_traffic::BatchMixMessageReceiver;
use client_core::client::mix_traffic::BatchMixMessageSender;
use client_core::client::mix_traffic::MixTrafficController;
use client_core::client::real_messages_control;
use client_core::client::real_messages_control::RealMessagesController;
use client_core::client::received_buffer::ReceivedBufferRequestReceiver;
use client_core::client::received_buffer::ReceivedMessagesBufferController;
use client_core::client::topology_control::TopologyAccessor;
use client_core::client::topology_control::TopologyRefresher;
use client_core::client::topology_control::TopologyRefresherConfig;
use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use gateway_client::AcknowledgementReceiver;
use gateway_client::AcknowledgementSender;
use gateway_client::GatewayClient;
use gateway_client::MixnetMessageReceiver;
use gateway_client::MixnetMessageSender;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::preparer::MessagePreparer;
use rand::rngs::OsRng;
use received_processor::ReceivedMessagesProcessor;
use std::sync::Arc;
use std::time::Duration;
use topology::{gateway, nym_topology_from_bonds, NymTopology};
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::{console_log, console_warn};

pub(crate) mod received_processor;

// TODO: make those properly configurable later
const ACK_WAIT_MULTIPLIER: f64 = 1.5;
const ACK_WAIT_ADDITION: Duration = Duration::from_millis(1_500);
const LOOP_COVER_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(200);
const MESSAGE_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(20);
const AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(50);
const AVERAGE_ACK_DELAY: Duration = Duration::from_millis(50);
const TOPOLOGY_REFRESH_RATE: Duration = Duration::from_secs(5 * 60);
const TOPOLOGY_RESOLUTION_TIMEOUT: Duration = Duration::from_millis(5_000);

const GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_millis(1_500);

#[wasm_bindgen]
pub struct NymClient {
    validator_server: Url,
    disabled_credentials_mode: bool,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,

    // message_preparer: Option<MessagePreparer<OsRng>>,
    // message_receiver: MessageReceiver,

    // TODO: this should be stored somewhere persistently
    // received_keys: HashSet<SURBEncryptionKey>,

    // TODO: only temporary
    topology: Option<NymTopology>,
    // gateway_client: Option<GatewayClient>,
    gateway_identity: Option<identity::PublicKey>,

    // callbacks
    on_message: Option<js_sys::Function>,
    on_gateway_connect: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl NymClient {
    #[wasm_bindgen(constructor)]
    pub fn new(validator_server: String) -> Self {
        let mut rng = OsRng;
        // for time being generate new keys each time...
        let mut key_manager = KeyManager::new(&mut rng);
        console_log!("generated new set of keys");

        Self {
            key_manager,
            validator_server: validator_server
                .parse()
                .expect("malformed validator server url provided"),
            // message_preparer: None,
            // received_keys: Default::default(),
            topology: None,
            // gateway_client: None,
            gateway_identity: None,
            on_message: None,
            on_gateway_connect: None,
            disabled_credentials_mode: true,
        }
    }

    pub fn set_on_message(&mut self, on_message: js_sys::Function) {
        self.on_message = Some(on_message);
    }

    pub fn set_on_gateway_connect(&mut self, on_connect: js_sys::Function) {
        console_log!("setting on connect...");
        self.on_gateway_connect = Some(on_connect)
    }

    pub fn set_disabled_credentials_mode(&mut self, disabled_credentials_mode: bool) {
        console_log!(
            "Setting disabled credentials mode to {}",
            disabled_credentials_mode
        );
        self.disabled_credentials_mode = disabled_credentials_mode;
    }

    fn as_mix_recipient(&self) -> Recipient {
        Recipient::new(
            *self.key_manager.identity_keypair().public_key(),
            *self.key_manager.encryption_keypair().public_key(),
            self.gateway_identity
                .expect("gateway connection was not established!"),
        )
    }

    pub fn self_address(&self) -> String {
        return "foomp".into();
        // self.as_mix_recipient().to_string()
    }

    // future constantly pumping loop cover traffic at some specified average rate
    // the pumped traffic goes to the MixTrafficController
    fn start_cover_traffic_stream(
        &self,
        topology_accessor: TopologyAccessor,
        mix_tx: BatchMixMessageSender,
    ) {
        console_log!("Starting loop cover traffic stream...");

        LoopCoverTrafficStream::new(
            self.key_manager.ack_key(),
            AVERAGE_ACK_DELAY,
            AVERAGE_PACKET_DELAY,
            LOOP_COVER_STREAM_AVERAGE_DELAY,
            mix_tx,
            self.as_mix_recipient(),
            topology_accessor,
        )
        .start();
    }

    fn start_real_traffic_controller(
        &self,
        topology_accessor: TopologyAccessor,
        ack_receiver: AcknowledgementReceiver,
        input_receiver: InputMessageReceiver,
        mix_sender: BatchMixMessageSender,
    ) {
        let controller_config = real_messages_control::Config::new(
            self.key_manager.ack_key(),
            ACK_WAIT_MULTIPLIER,
            ACK_WAIT_ADDITION,
            AVERAGE_ACK_DELAY,
            MESSAGE_STREAM_AVERAGE_DELAY,
            AVERAGE_PACKET_DELAY,
            self.as_mix_recipient(),
        );

        console_log!("Starting real traffic stream...");

        RealMessagesController::new(
            controller_config,
            ack_receiver,
            input_receiver,
            mix_sender,
            topology_accessor,
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
        let gateway_owner = "n1kymvkx6vsq7pvn6hfurkpg06h3j4gxj4em7tlg".into();
        let gateway_id = "E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM".to_string();

        // TODO: might need a port
        let gateway_address = "ws://213.219.38.119:9000".into();
        // let gateway_address = "213.219.38.119".into();

        // for now there are no configs, etc.
        // let gateway_id = self.config.get_base().get_gateway_id();
        // if gateway_id.is_empty() {
        //     panic!("The identity of the gateway is unknown - did you run `nym-client` init?")
        // }
        // let gateway_owner = self.config.get_base().get_gateway_owner();
        // if gateway_owner.is_empty() {
        //     panic!("The owner of the gateway is unknown - did you run `nym-client` init?")
        // }
        // let gateway_address = self.config.get_base().get_gateway_listener();
        // if gateway_address.is_empty() {
        //     panic!("The address of the gateway is unknown - did you run `nym-client` init?")
        // }

        let gateway_identity = identity::PublicKey::from_base58_string(gateway_id)
            .expect("provided gateway id is invalid!");

        self.force_update_internal_topology().await;
        let gateway = self.choose_gateway();

        let mut gateway_client = GatewayClient::new(
            gateway_address,
            self.key_manager.identity_keypair(),
            gateway_identity,
            gateway_owner,
            Some(self.key_manager.gateway_shared_key()),
            mixnet_message_sender,
            ack_sender,
            GATEWAY_RESPONSE_TIMEOUT,
            None,
        );

        gateway_client.set_disabled_credentials_mode(self.disabled_credentials_mode);

        gateway_client
            .authenticate_and_start()
            .await
            .expect("could not authenticate and start up the gateway connection");

        match self.on_gateway_connect.as_ref() {
            Some(callback) => {
                callback
                    .call0(&JsValue::null())
                    .expect("on connect callback failed!");
            }
            None => console_log!("Gateway connection established - no callback specified"),
        };

        self.gateway_identity = Some(gateway_client.gateway_identity());

        gateway_client
    }

    // future responsible for periodically polling directory server and updating
    // the current global view of topology
    async fn start_topology_refresher(&mut self, topology_accessor: TopologyAccessor) {
        let topology_refresher_config = TopologyRefresherConfig::new(
            vec![self.validator_server.clone()],
            TOPOLOGY_REFRESH_RATE,
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
        topology_refresher.start();
    }

    // controller for sending sphinx packets to mixnet (either real traffic or cover traffic)
    // TODO: if we want to send control messages to gateway_client, this CAN'T take the ownership
    // over it. Perhaps GatewayClient needs to be thread-shareable or have some channel for
    // requests?
    fn start_mix_traffic_controller(
        &mut self,
        mix_rx: BatchMixMessageReceiver,
        gateway_client: GatewayClient,
    ) {
        console_log!("Starting mix traffic controller...");
        MixTrafficController::new(mix_rx, gateway_client).start();
    }

    pub async fn start(mut self) -> NymClient {
        // println!("hello world print");
        // console_log!("hello world log");
        // self
        console_log!("Starting nym client");
        // channels for inter-component communication
        // TODO: make the channels be internally created by the relevant components
        // rather than creating them here, so say for example the buffer controller would create the request channels
        // and would allow anyone to clone the sender channel

        // sphinx_message_sender is the transmitter for any component generating sphinx packets that are to be sent to the mixnet
        // they are used by cover traffic stream and real traffic stream
        // sphinx_message_receiver is the receiver used by MixTrafficController that sends the actual traffic
        let (sphinx_message_sender, sphinx_message_receiver) = mpsc::unbounded();

        // unwrapped_sphinx_sender is the transmitter of mixnet messages received from the gateway
        // unwrapped_sphinx_receiver is the receiver for said messages - used by ReceivedMessagesBuffer
        let (mixnet_messages_sender, mixnet_messages_receiver) = mpsc::unbounded();

        // used for announcing connection or disconnection of a channel for pushing re-assembled messages to
        let (received_buffer_request_sender, received_buffer_request_receiver) = mpsc::unbounded();

        // channels responsible for controlling real messages
        let (input_sender, input_receiver) = mpsc::unbounded::<InputMessage>();

        // channels responsible for controlling ack messages
        let (ack_sender, ack_receiver) = mpsc::unbounded();
        let shared_topology_accessor = TopologyAccessor::new();

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

        self.start_mix_traffic_controller(sphinx_message_receiver, gateway_client);
        self.start_real_traffic_controller(
            shared_topology_accessor.clone(),
            ack_receiver,
            input_receiver,
            sphinx_message_sender.clone(),
        );

        self.start_cover_traffic_stream(shared_topology_accessor, sphinx_message_sender);
        self
    }

    // Right now it's impossible to have async exported functions to take `&self` rather than self
    pub async fn initial_setup(self) -> Self {
        // let disabled_credentials_mode = self.disabled_credentials_mode;
        //
        // let bandwidth_controller = None;
        //
        // let mut client = self.get_and_update_topology().await;
        // let gateway = client.choose_gateway();
        //
        // let (mixnet_messages_sender, mixnet_messages_receiver) = mpsc::unbounded();
        // let (ack_sender, ack_receiver) = mpsc::unbounded();
        //
        // let mut gateway_client = GatewayClient::new(
        //     gateway.clients_address(),
        //     Arc::clone(&client.identity),
        //     gateway.identity_key,
        //     gateway.owner.clone(),
        //     None,
        //     mixnet_messages_sender,
        //     ack_sender,
        //     GATEWAY_RESPONSE_TIMEOUT,
        //     bandwidth_controller,
        // );
        //
        // gateway_client.set_disabled_credentials_mode(disabled_credentials_mode);
        //
        // gateway_client
        //     .authenticate_and_start()
        //     .await
        //     .expect("could not authenticate and start up the gateway connection");
        //
        // client.gateway_client = Some(gateway_client);
        // match client.on_gateway_connect.as_ref() {
        //     Some(callback) => {
        //         callback
        //             .call0(&JsValue::null())
        //             .expect("on connect callback failed!");
        //     }
        //     None => console_log!("Gateway connection established - no callback specified"),
        // };
        //
        // let rng = rand::rngs::OsRng;
        // let message_preparer = MessagePreparer::new(
        //     rng,
        //     client.self_recipient(),
        //     AVERAGE_PACKET_DELAY,
        //     AVERAGE_ACK_DELAY,
        // );
        //
        // let received_processor = ReceivedMessagesProcessor::new(
        //     Arc::clone(&client.encryption_keys),
        //     Arc::clone(&client.ack_key),
        // );
        //
        // client.message_preparer = Some(message_preparer);
        //
        // spawn_local(received_processor.start_processing(
        //     mixnet_messages_receiver,
        //     ack_receiver,
        //     client.on_message.take().expect("on_message was not set!"),
        // ));
        //
        self
    }

    // Right now it's impossible to have async exported functions to take `&mut self` rather than mut self
    // TODO: try Rc<RefCell<Self>> approach?
    pub async fn send_message(mut self, message: String, recipient: String) -> Self {
        console_log!("Sending {} to {}", message, recipient);

        todo!()

        // let message_bytes = message.into_bytes();
        // let recipient = Recipient::try_from_base58_string(recipient).unwrap();
        //
        // let topology = self
        //     .topology
        //     .as_ref()
        //     .expect("did not obtain topology before");
        //
        // let message_preparer = self.message_preparer.as_mut().unwrap();
        //
        // let (split_message, _reply_keys) = message_preparer
        //     .prepare_and_split_message(message_bytes, false, topology)
        //     .expect("failed to split the message");
        //
        // let mut mix_packets = Vec::with_capacity(split_message.len());
        // for message_chunk in split_message {
        //     // don't bother with acks etc. for time being
        //     let prepared_fragment = message_preparer
        //         .prepare_chunk_for_sending(message_chunk, topology, &self.ack_key, &recipient)
        //         .unwrap();
        //
        //     console_warn!("packet is going to have round trip time of {:?}, but we're not going to do anything for acks anyway ", prepared_fragment.total_delay);
        //     mix_packets.push(prepared_fragment.mix_packet);
        // }
        // self.gateway_client
        //     .as_mut()
        //     .unwrap()
        //     .batch_send_mix_packets(mix_packets)
        //     .await
        //     .unwrap();
        // self
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
        self.force_update_internal_topology().await;
        self
    }

    pub(crate) async fn force_update_internal_topology(&mut self) {
        let new_topology = self.get_nym_topology().await;
        self.update_topology(new_topology);
    }

    pub(crate) fn update_topology(&mut self, topology: NymTopology) {
        self.topology = Some(topology)
    }

    // // when updated to 0.10.0, to prevent headache later on, this function requires those two imports:
    // // use js_sys::Promise;
    // // use wasm_bindgen_futures::future_to_promise;
    // //
    // // pub fn get_full_topology_json(&self) -> Promise {
    // //     let validator_client_config = validator_client::Config::new(
    // //         vec![self.validator_server.clone()],
    // //         &self.mixnet_contract_address,
    // //     );
    // //     let validator_client = validator_client::Client::new(validator_client_config);
    // //
    // //     future_to_promise(async move {
    // //         let topology = &validator_client.get_active_topology().await.unwrap();
    // //         Ok(JsValue::from_serde(&topology).unwrap())
    // //     })
    // // }

    pub(crate) async fn get_nym_topology(&self) -> NymTopology {
        let validator_client = validator_client::ApiClient::new(self.validator_server.clone());

        let mixnodes = match validator_client.get_cached_active_mixnodes().await {
            Err(err) => panic!("{:?}", err),
            Ok(mixes) => mixes,
        };

        let gateways = match validator_client.get_cached_gateways().await {
            Err(err) => panic!("{}", err),
            Ok(gateways) => gateways,
        };

        let topology = nym_topology_from_bonds(mixnodes, gateways);
        let version = env!("CARGO_PKG_VERSION");
        topology.filter_system_version(version)
    }
}
