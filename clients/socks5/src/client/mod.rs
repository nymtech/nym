// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use crate::error::Socks5ClientError;
use crate::socks;
use crate::socks::{
    authentication::{AuthenticationMethods, Authenticator, User},
    server::SphinxSocksServer,
};
use client_connections::{ConnectionCommandReceiver, ConnectionCommandSender, LaneQueueLengths};
use client_core::client::cover_traffic_stream::LoopCoverTrafficStream;
use client_core::client::inbound_messages::{
    InputMessage, InputMessageReceiver, InputMessageSender,
};
use client_core::client::key_manager::KeyManager;
use client_core::client::mix_traffic::{BatchMixMessageSender, MixTrafficController};
use client_core::client::real_messages_control;
use client_core::client::real_messages_control::RealMessagesController;
use client_core::client::received_buffer::{
    ReceivedBufferRequestReceiver, ReceivedBufferRequestSender, ReceivedMessagesBufferController,
};
use client_core::client::replies::reply_controller;
use client_core::client::replies::reply_controller::{
    ReplyControllerReceiver, ReplyControllerSender,
};
use client_core::client::replies::reply_storage::{
    fs_backend, CombinedReplyStorage, PersistentReplyStorage, SentReplyKeys,
};
use client_core::client::topology_control::{
    TopologyAccessor, TopologyRefresher, TopologyRefresherConfig,
};
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use client_core::error::ClientCoreError;
use crypto::asymmetric::identity;
use futures::channel::mpsc;
use futures::StreamExt;
use gateway_client::bandwidth::BandwidthController;
use gateway_client::{
    AcknowledgementReceiver, AcknowledgementSender, GatewayClient, MixnetMessageReceiver,
    MixnetMessageSender,
};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::addressing::nodes::NodeIdentity;
use std::sync::atomic::Ordering;
use task::{wait_for_signal, ShutdownListener, ShutdownNotifier};

pub mod config;

// Channels used to control the main task from outside
pub type Socks5ControlMessageSender = mpsc::UnboundedSender<Socks5ControlMessage>;
pub type Socks5ControlMessageReceiver = mpsc::UnboundedReceiver<Socks5ControlMessage>;

#[derive(Debug)]
pub enum Socks5ControlMessage {
    /// Tell the main task to stop
    Stop,
}

pub struct NymClient {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,
}

impl NymClient {
    pub fn new(config: Config) -> Self {
        let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
        let key_manager = KeyManager::load_keys(&pathfinder).expect("failed to load stored keys");

        NymClient {
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
        shutdown: ShutdownListener,
    ) {
        info!("Starting loop cover traffic stream...");

        let mut stream = LoopCoverTrafficStream::new(
            self.key_manager.ack_key(),
            self.config.get_base().get_average_ack_delay(),
            self.config.get_base().get_average_packet_delay(),
            self.config
                .get_base()
                .get_loop_cover_traffic_average_delay(),
            mix_tx,
            self.as_mix_recipient(),
            topology_accessor,
        );

        if let Some(size) = self.config.get_base().get_use_extended_packet_size() {
            log::debug!("Setting extended packet size: {:?}", size);
            stream.set_custom_packet_size(size.into());
        }

        stream.start_with_shutdown(shutdown);
    }

    #[allow(clippy::too_many_arguments)]
    fn start_real_traffic_controller(
        &self,
        topology_accessor: TopologyAccessor,
        ack_receiver: AcknowledgementReceiver,
        input_receiver: InputMessageReceiver,
        mix_sender: BatchMixMessageSender,
        reply_storage: CombinedReplyStorage,
        reply_controller_sender: ReplyControllerSender,
        reply_controller_receiver: ReplyControllerReceiver,
        client_connection_rx: ConnectionCommandReceiver,
        lane_queue_lengths: LaneQueueLengths,
        shutdown: ShutdownListener,
    ) {
        let mut controller_config = real_messages_control::Config::new(
            self.config.get_debug_settings(),
            self.key_manager.ack_key(),
            self.as_mix_recipient(),
        );

        if let Some(size) = self.config.get_base().get_use_extended_packet_size() {
            log::debug!("Setting extended packet size: {:?}", size);
            controller_config.set_custom_packet_size(size.into());
        }

        info!("Starting real traffic stream...");

        RealMessagesController::new(
            controller_config,
            ack_receiver,
            input_receiver,
            mix_sender,
            topology_accessor,
            reply_storage,
            reply_controller_sender,
            reply_controller_receiver,
            lane_queue_lengths,
            client_connection_rx,
        )
        .start_with_shutdown(shutdown);
    }

    // buffer controlling all messages fetched from provider
    // required so that other components would be able to use them (say the websocket)
    fn start_received_messages_buffer_controller(
        &self,
        query_receiver: ReceivedBufferRequestReceiver,
        mixnet_receiver: MixnetMessageReceiver,
        reply_key_storage: SentReplyKeys,
        reply_controller_sender: ReplyControllerSender,
        shutdown: ShutdownListener,
    ) {
        info!("Starting received messages buffer controller...");
        ReceivedMessagesBufferController::new(
            self.key_manager.encryption_keypair(),
            query_receiver,
            mixnet_receiver,
            reply_key_storage,
            reply_controller_sender,
        )
        .start_with_shutdown(shutdown)
    }

    async fn start_gateway_client(
        &mut self,
        mixnet_message_sender: MixnetMessageSender,
        ack_sender: AcknowledgementSender,
        shutdown: ShutdownListener,
    ) -> GatewayClient {
        let gateway_id = self.config.get_base().get_gateway_id();
        if gateway_id.is_empty() {
            panic!("The identity of the gateway is unknown - did you run `nym-client` init?")
        }
        let gateway_owner = self.config.get_base().get_gateway_owner();
        if gateway_owner.is_empty() {
            panic!("The owner of the gateway is unknown - did you run `nym-client` init?")
        }
        let gateway_address = self.config.get_base().get_gateway_listener();
        if gateway_address.is_empty() {
            panic!("The address of the gateway is unknown - did you run `nym-client` init?")
        }

        let gateway_identity = identity::PublicKey::from_base58_string(gateway_id)
            .expect("provided gateway id is invalid!");

        #[cfg(feature = "coconut")]
        let bandwidth_controller = {
            let details = network_defaults::NymNetworkDetails::new_from_env();
            let client_config = validator_client::Config::try_from_nym_network_details(&details)
                .expect("failed to construct validator client config");
            let client = validator_client::Client::new_query(client_config)
                .expect("Could not construct query client");
            let coconut_api_clients =
                validator_client::CoconutApiClient::all_coconut_api_clients(&client)
                    .await
                    .expect("Could not query api clients");
            BandwidthController::new(
                credential_storage::initialise_storage(self.config.get_base().get_database_path())
                    .await,
                coconut_api_clients,
            )
        };
        #[cfg(not(feature = "coconut"))]
        let bandwidth_controller = BandwidthController::new(
            credential_storage::initialise_storage(self.config.get_base().get_database_path())
                .await,
        )
        .expect("Could not create bandwidth controller");

        let mut gateway_client = GatewayClient::new(
            gateway_address,
            self.key_manager.identity_keypair(),
            gateway_identity,
            gateway_owner,
            Some(self.key_manager.gateway_shared_key()),
            mixnet_message_sender,
            ack_sender,
            self.config.get_base().get_gateway_response_timeout(),
            Some(bandwidth_controller),
            Some(shutdown),
        );

        gateway_client
            .set_disabled_credentials_mode(self.config.get_base().get_disabled_credentials_mode());

        gateway_client
            .authenticate_and_start()
            .await
            .expect("could not authenticate and start up the gateway connection");

        gateway_client
    }

    // future responsible for periodically polling directory server and updating
    // the current global view of topology
    async fn start_topology_refresher(
        &mut self,
        topology_accessor: TopologyAccessor,
        shutdown: ShutdownListener,
    ) -> Result<(), Socks5ClientError> {
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
        topology_refresher.refresh().await;

        // TODO: a slightly more graceful termination here
        if !topology_refresher.is_topology_routable().await {
            log::error!(
                "The current network topology seem to be insufficient to route any packets through \
                - check if enough nodes and a gateway are online"
            );
            return Err(ClientCoreError::InsufficientNetworkTopology.into());
        }

        info!("Starting topology refresher...");
        topology_refresher.start_with_shutdown(shutdown);
        Ok(())
    }

    // controller for sending sphinx packets to mixnet (either real traffic or cover traffic)
    // TODO: if we want to send control messages to gateway_client, this CAN'T take the ownership
    // over it. Perhaps GatewayClient needs to be thread-shareable or have some channel for
    // requests?
    fn start_mix_traffic_controller(
        gateway_client: GatewayClient,
        shutdown: ShutdownListener,
    ) -> BatchMixMessageSender {
        info!("Starting mix traffic controller...");
        let (mix_traffic_controller, mix_tx) = MixTrafficController::new(gateway_client);
        mix_traffic_controller.start_with_shutdown(shutdown);
        mix_tx
    }

    async fn setup_persistent_reply_storage(
        &self,
        shutdown: ShutdownListener,
    ) -> Result<CombinedReplyStorage, Socks5ClientError> {
        // if the database file doesnt exist, initialise fresh storage, otherwise attempt to load the existing one
        let db_path = self.config.get_base().get_reply_surb_database_path();
        let (persistent_storage, mem_store) = if db_path.exists() {
            info!("loading existing surb database");
            let storage_backend = match fs_backend::Backend::try_load(db_path).await {
                Ok(backend) => backend,
                Err(err) => {
                    error!("failed to setup persistent storage backend for our reply needs: {err}");
                    return Err(err.into());
                }
            };
            let persistent_storage = PersistentReplyStorage::new(storage_backend);
            let mem_store = persistent_storage.load_state_from_backend().await?;
            (persistent_storage, mem_store)
        } else {
            info!("creating fresh surb database");
            let storage_backend = match fs_backend::Backend::init(db_path).await {
                Ok(backend) => backend,
                Err(err) => {
                    error!("failed to setup persistent storage backend for our reply needs: {err}");
                    return Err(err.into());
                }
            };
            let persistent_storage = PersistentReplyStorage::new(storage_backend);
            let mem_store = CombinedReplyStorage::new(
                self.config
                    .get_base()
                    .get_minimum_reply_surb_storage_threshold(),
                self.config
                    .get_base()
                    .get_maximum_reply_surb_storage_threshold(),
            );
            (persistent_storage, mem_store)
        };

        let store_clone = mem_store.clone();
        tokio::spawn(async move {
            persistent_storage
                .flush_on_shutdown(store_clone, shutdown)
                .await
        });

        Ok(mem_store)
    }

    fn start_socks5_listener(
        &self,
        buffer_requester: ReceivedBufferRequestSender,
        msg_input: InputMessageSender,
        client_connection_tx: ConnectionCommandSender,
        lane_queue_lengths: LaneQueueLengths,
        shutdown: ShutdownListener,
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
            lane_queue_lengths,
            socks::client::Config::new(
                self.config.get_send_anonymously(),
                self.config.get_connection_start_surbs(),
                self.config.get_per_request_surbs(),
            ),
            shutdown,
        );
        tokio::spawn(async move {
            sphinx_socks
                .serve(msg_input, buffer_requester, client_connection_tx)
                .await
        });
    }

    /// blocking version of `start` method. Will run forever (or until SIGINT is sent)
    pub async fn run_forever(&mut self) -> Result<(), Socks5ClientError> {
        let mut shutdown = self.start().await?;
        wait_for_signal().await;

        log::info!("Sending shutdown");
        client_core::client::SHUTDOWN_HAS_BEEN_SIGNALLED.store(true, Ordering::Relaxed);
        shutdown.signal_shutdown().ok();

        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        shutdown.wait_for_shutdown().await;

        log::info!("Stopping nym-socks5-client");
        Ok(())
    }

    // Variant of `run_forever` that listends for remote control messages
    pub async fn run_and_listen(
        &mut self,
        mut receiver: Socks5ControlMessageReceiver,
    ) -> Result<(), Socks5ClientError> {
        let mut shutdown = self.start().await?;
        tokio::select! {
            message = receiver.next() => {
                log::debug!("Received message: {:?}", message);
                match message {
                    Some(Socks5ControlMessage::Stop) => {
                        log::info!("Received stop message");
                    }
                    None => {
                        log::info!("Channel closed, stopping");
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                log::info!("Received SIGINT");
            },
        }

        log::info!("Sending shutdown");
        client_core::client::SHUTDOWN_HAS_BEEN_SIGNALLED.store(true, Ordering::Relaxed);
        shutdown.signal_shutdown().ok();

        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        shutdown.wait_for_shutdown().await;

        log::info!("Stopping nym-socks5-client");
        Ok(())
    }

    pub async fn start(&mut self) -> Result<ShutdownNotifier, Socks5ClientError> {
        info!("Starting nym client");
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

        // let reply_key_storage =
        //     ReplyKeyStorage::load(self.config.get_base().get_reply_encryption_key_store_path())
        //         .expect("Failed to load reply key storage!");

        // channels responsible for dealing with reply-related fun
        let (reply_controller_sender, reply_controller_receiver) =
            reply_controller::new_control_channels();

        // Shutdown notifier for signalling tasks to stop
        let shutdown = ShutdownNotifier::default();

        let reply_storage = self
            .setup_persistent_reply_storage(shutdown.subscribe())
            .await?;

        // the components are started in very specific order. Unless you know what you are doing,
        // do not change that.
        self.start_topology_refresher(shared_topology_accessor.clone(), shutdown.subscribe())
            .await?;
        self.start_received_messages_buffer_controller(
            received_buffer_request_receiver,
            mixnet_messages_receiver,
            reply_storage.key_storage(),
            reply_controller_sender.clone(),
            shutdown.subscribe(),
        );

        let gateway_client = self
            .start_gateway_client(mixnet_messages_sender, ack_sender, shutdown.subscribe())
            .await;

        // The sphinx_message_sender is the transmitter for any component generating sphinx packets
        // that are to be sent to the mixnet. They are used by cover traffic stream and real
        // traffic stream.
        // The MixTrafficController then sends the actual traffic
        let sphinx_message_sender =
            Self::start_mix_traffic_controller(gateway_client, shutdown.subscribe());

        // Channel for announcing closed (socks5) connections by the controller.
        // This will be forwarded to `OutQueueControl`
        let (client_connection_tx, client_connection_rx) = mpsc::unbounded();

        // Shared queue length data. Published by the `OutQueueController` in the client, and used
        // primarily to throttle incoming connections
        let shared_lane_queue_lengths = LaneQueueLengths::new();

        self.start_real_traffic_controller(
            shared_topology_accessor.clone(),
            ack_receiver,
            input_receiver,
            sphinx_message_sender.clone(),
            reply_storage,
            reply_controller_sender,
            reply_controller_receiver,
            client_connection_rx,
            shared_lane_queue_lengths.clone(),
            shutdown.subscribe(),
        );

        if !self
            .config
            .get_base()
            .get_disabled_loop_cover_traffic_stream()
        {
            self.start_cover_traffic_stream(
                shared_topology_accessor,
                sphinx_message_sender,
                shutdown.subscribe(),
            );
        }

        self.start_socks5_listener(
            received_buffer_request_sender,
            input_sender,
            client_connection_tx,
            shared_lane_queue_lengths,
            shutdown.subscribe(),
        );

        info!("Client startup finished!");
        info!("The address of this client is: {}", self.as_mix_recipient());

        Ok(shutdown)
    }
}
