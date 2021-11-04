// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use log::*;
use tokio::runtime::Runtime;

use client_core::client::cover_traffic_stream::LoopCoverTrafficStream;
use client_core::client::inbound_messages::{
    InputMessage, InputMessageReceiver, InputMessageSender,
};
use client_core::client::key_manager::KeyManager;
use client_core::client::mix_traffic::{
    BatchMixMessageReceiver, BatchMixMessageSender, MixTrafficController,
};
use client_core::client::real_messages_control::RealMessagesController;
use client_core::client::received_buffer::{
    ReceivedBufferRequestReceiver, ReceivedBufferRequestSender, ReceivedMessagesBufferController,
};
use client_core::client::reply_key_storage::ReplyKeyStorage;
use client_core::client::topology_control::{
    TopologyAccessor, TopologyRefresher, TopologyRefresherConfig,
};
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
#[cfg(feature = "coconut")]
use coconut_interface::{hash_to_scalar, Credential, Parameters};
#[cfg(feature = "coconut")]
use credentials::bandwidth::{
    prepare_for_spending, BandwidthVoucherAttributes, BANDWIDTH_VALUE, TOTAL_ATTRIBUTES,
};
#[cfg(feature = "coconut")]
use credentials::obtain_aggregate_verification_key;
use crypto::asymmetric::identity;
use gateway_client::{
    AcknowledgementReceiver, AcknowledgementSender, GatewayClient, MixnetMessageReceiver,
    MixnetMessageSender,
};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::addressing::nodes::NodeIdentity;

use crate::client::config::Config;
use crate::socks::{
    authentication::{AuthenticationMethods, Authenticator, User},
    server::SphinxSocksServer,
};

pub(crate) mod config;

pub struct NymClient {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,

    /// Tokio runtime used for futures execution.
    // TODO: JS: Personally I think I prefer the implicit way of using it that we've done with the
    // gateway.
    runtime: Runtime,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,
}

impl NymClient {
    pub fn new(config: Config) -> Self {
        let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
        let key_manager = KeyManager::load_keys(&pathfinder).expect("failed to load stored keys");

        NymClient {
            runtime: Runtime::new().unwrap(),
            config,
            key_manager,
        }
    }

    pub fn as_mix_recipient(&self) -> Recipient {
        Recipient::new(
            *self.key_manager.identity_keypair().public_key(),
            *self.key_manager.encryption_keypair().public_key(),
            // TODO: below only works under assumption that gateway address == gateway id
            // (which currently is true)
            NodeIdentity::from_base58_string(self.config.get_base().get_gateway_id()).unwrap(),
        )
    }

    // future constantly pumping loop cover traffic at some specified average rate
    // the pumped traffic goes to the MixTrafficController
    fn start_cover_traffic_stream(
        &self,
        topology_accessor: TopologyAccessor,
        mix_tx: BatchMixMessageSender,
    ) {
        info!("Starting loop cover traffic stream...");
        // we need to explicitly enter runtime due to "next_delay: time::delay_for(Default::default())"
        // set in the constructor which HAS TO be called within context of a tokio runtime
        let _guard = self.runtime.enter();

        LoopCoverTrafficStream::new(
            self.key_manager.ack_key(),
            self.config.get_base().get_average_ack_delay(),
            self.config.get_base().get_average_packet_delay(),
            self.config
                .get_base()
                .get_loop_cover_traffic_average_delay(),
            mix_tx,
            self.as_mix_recipient(),
            topology_accessor,
        )
        .start(self.runtime.handle());
    }

    fn start_real_traffic_controller(
        &self,
        topology_accessor: TopologyAccessor,
        reply_key_storage: ReplyKeyStorage,
        ack_receiver: AcknowledgementReceiver,
        input_receiver: InputMessageReceiver,
        mix_sender: BatchMixMessageSender,
    ) {
        let controller_config = client_core::client::real_messages_control::Config::new(
            self.key_manager.ack_key(),
            self.config.get_base().get_ack_wait_multiplier(),
            self.config.get_base().get_ack_wait_addition(),
            self.config.get_base().get_average_ack_delay(),
            self.config.get_base().get_message_sending_average_delay(),
            self.config.get_base().get_average_packet_delay(),
            self.as_mix_recipient(),
        );

        info!("Starting real traffic stream...");
        // we need to explicitly enter runtime due to "next_delay: time::delay_for(Default::default())"
        // set in the constructor [of OutQueueControl] which HAS TO be called within context of a tokio runtime
        // When refactoring this restriction should definitely be removed.
        let _guard = self.runtime.enter();

        RealMessagesController::new(
            controller_config,
            ack_receiver,
            input_receiver,
            mix_sender,
            topology_accessor,
            reply_key_storage,
        )
        .start(self.runtime.handle());
    }

    // buffer controlling all messages fetched from provider
    // required so that other components would be able to use them (say the websocket)
    fn start_received_messages_buffer_controller(
        &self,
        query_receiver: ReceivedBufferRequestReceiver,
        mixnet_receiver: MixnetMessageReceiver,
        reply_key_storage: ReplyKeyStorage,
    ) {
        info!("Starting received messages buffer controller...");
        ReceivedMessagesBufferController::new(
            self.key_manager.encryption_keypair(),
            query_receiver,
            mixnet_receiver,
            reply_key_storage,
        )
        .start(self.runtime.handle())
    }

    #[cfg(feature = "coconut")]
    async fn prepare_coconut_credential(&self) -> Credential {
        let verification_key = obtain_aggregate_verification_key(
            &self.config.get_base().get_validator_api_endpoints(),
        )
        .await
        .expect("could not obtain aggregate verification key of validators");

        let params = Parameters::new(TOTAL_ATTRIBUTES).unwrap();
        let bandwidth_credential_attributes = BandwidthVoucherAttributes {
            serial_number: params.random_scalar(),
            binding_number: params.random_scalar(),
            voucher_value: hash_to_scalar(BANDWIDTH_VALUE.to_be_bytes()),
            voucher_info: hash_to_scalar(String::from("BandwidthVoucher").as_bytes()),
        };

        let bandwidth_credential = credentials::bandwidth::obtain_signature(
            &params,
            &bandwidth_credential_attributes,
            &self.config.get_base().get_validator_api_endpoints(),
        )
        .await
        .expect("could not obtain bandwidth credential");
        // the above would presumably be loaded from a file

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)
        prepare_for_spending(
            &self.key_manager.identity_keypair().public_key().to_bytes(),
            &bandwidth_credential,
            &bandwidth_credential_attributes,
            &verification_key,
        )
        .expect("could not prepare out bandwidth credential for spending")
    }

    fn start_gateway_client(
        &mut self,
        mixnet_message_sender: MixnetMessageSender,
        ack_sender: AcknowledgementSender,
    ) -> GatewayClient {
        let gateway_id = self.config.get_base().get_gateway_id();
        if gateway_id.is_empty() {
            panic!("The identity of the gateway is unknown - did you run `nym-client` init?")
        }
        let gateway_address = self.config.get_base().get_gateway_listener();
        if gateway_address.is_empty() {
            panic!("The address of the gateway is unknown - did you run `nym-client` init?")
        }

        let gateway_identity = identity::PublicKey::from_base58_string(gateway_id)
            .expect("provided gateway id is invalid!");

        self.runtime.block_on(async {
            #[cfg(feature = "coconut")]
            let coconut_credential = self.prepare_coconut_credential().await;

            let mut gateway_client = GatewayClient::new(
                gateway_address,
                self.key_manager.identity_keypair(),
                gateway_identity,
                Some(self.key_manager.gateway_shared_key()),
                mixnet_message_sender,
                ack_sender,
                self.config.get_base().get_gateway_response_timeout(),
            );

            gateway_client
                .authenticate_and_start(
                    #[cfg(feature = "coconut")]
                    Some(coconut_credential),
                )
                .await
                .expect("could not authenticate and start up the gateway connection");

            gateway_client
        })
    }

    // future responsible for periodically polling directory server and updating
    // the current global view of topology
    fn start_topology_refresher(&mut self, topology_accessor: TopologyAccessor) {
        let topology_refresher_config = TopologyRefresherConfig::new(
            self.config.get_base().get_validator_api_endpoints(),
            self.config.get_base().get_topology_refresh_rate(),
            env!("CARGO_PKG_VERSION").to_string(),
        );
        let mut topology_refresher =
            TopologyRefresher::new(topology_refresher_config, topology_accessor);
        // before returning, block entire runtime to refresh the current network view so that any
        // components depending on topology would see a non-empty view
        info!("Obtaining initial network topology");
        self.runtime.block_on(topology_refresher.refresh());

        // TODO: a slightly more graceful termination here
        if !self
            .runtime
            .block_on(topology_refresher.is_topology_routable())
        {
            panic!(
                "The current network topology seem to be insufficient to route any packets through\
                - check if enough nodes and a gateway are online"
            );
        }

        info!("Starting topology refresher...");
        topology_refresher.start(self.runtime.handle());
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
        info!("Starting mix traffic controller...");
        MixTrafficController::new(mix_rx, gateway_client).start(self.runtime.handle());
    }

    fn start_socks5_listener(
        &self,
        buffer_requester: ReceivedBufferRequestSender,
        msg_input: InputMessageSender,
    ) {
        info!("Starting socks5 listener...");
        let auth_methods = vec![AuthenticationMethods::NoAuth as u8];
        let allowed_users: Vec<User> = Vec::new();

        let authenticator = Authenticator::new(auth_methods, allowed_users);
        let mut sphinx_socks = SphinxSocksServer::new(
            self.config.get_listening_port(),
            authenticator,
            self.config.get_provider_mix_address(),
            self.as_mix_recipient(),
        );
        self.runtime
            .spawn(async move { sphinx_socks.serve(msg_input, buffer_requester).await });
    }

    /// blocking version of `start` method. Will run forever (or until SIGINT is sent)
    pub fn run_forever(&mut self) {
        self.start();
        if let Err(e) = self.runtime.block_on(tokio::signal::ctrl_c()) {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }

        println!(
            "Received SIGINT - the client will terminate now (threads are not yet nicely stopped, if you see stack traces that's alright)."
        );
    }

    pub fn start(&mut self) {
        info!("Starting nym client");
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

        let reply_key_storage =
            ReplyKeyStorage::load(self.config.get_base().get_reply_encryption_key_store_path())
                .expect("Failed to load reply key storage!");

        // the components are started in very specific order. Unless you know what you are doing,
        // do not change that.
        self.start_topology_refresher(shared_topology_accessor.clone());
        self.start_received_messages_buffer_controller(
            received_buffer_request_receiver,
            mixnet_messages_receiver,
            reply_key_storage.clone(),
        );

        let gateway_client = self.start_gateway_client(mixnet_messages_sender, ack_sender);

        self.start_mix_traffic_controller(sphinx_message_receiver, gateway_client);
        self.start_real_traffic_controller(
            shared_topology_accessor.clone(),
            reply_key_storage,
            ack_receiver,
            input_receiver,
            sphinx_message_sender.clone(),
        );

        self.start_cover_traffic_stream(shared_topology_accessor, sphinx_message_sender);
        self.start_socks5_listener(received_buffer_request_sender, input_sender);

        info!("Client startup finished!");
        info!("The address of this client is: {}", self.as_mix_recipient());
    }
}
