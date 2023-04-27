// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::received_buffer::ReceivedBufferMessage;
use crate::client::cover_traffic_stream::LoopCoverTrafficStream;
use crate::client::inbound_messages::{InputMessage, InputMessageReceiver, InputMessageSender};
use crate::client::key_manager::KeyManager;
use crate::client::mix_traffic::{BatchMixMessageSender, MixTrafficController};
use crate::client::real_messages_control;
use crate::client::real_messages_control::RealMessagesController;
use crate::client::received_buffer::{
    ReceivedBufferRequestReceiver, ReceivedBufferRequestSender, ReceivedMessagesBufferController,
};
use crate::client::replies::reply_controller;
use crate::client::replies::reply_controller::{ReplyControllerReceiver, ReplyControllerSender};
use crate::client::replies::reply_storage::{
    CombinedReplyStorage, PersistentReplyStorage, ReplyStorageBackend, SentReplyKeys,
};
use crate::client::topology_control::nym_api_provider::NymApiTopologyProvider;
use crate::client::topology_control::{
    TopologyAccessor, TopologyRefresher, TopologyRefresherConfig,
};
use crate::config::{Config, DebugConfig, GatewayEndpointConfig};
use crate::error::ClientCoreError;
use crate::{config, spawn_future};
use futures::channel::mpsc;
use log::{debug, info};
use nym_bandwidth_controller::BandwidthController;
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway_client::{
    AcknowledgementReceiver, AcknowledgementSender, GatewayClient, MixnetMessageReceiver,
    MixnetMessageSender,
};
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_sphinx::receiver::{ReconstructedMessage, SphinxMessageReceiver};
use nym_task::connections::{ConnectionCommandReceiver, ConnectionCommandSender, LaneQueueLengths};
use nym_task::{TaskClient, TaskManager};
use nym_topology::provider_trait::TopologyProvider;
use std::sync::Arc;
use tap::TapFallible;
use url::Url;

use nym_credential_storage::storage::Storage;
#[cfg(not(target_arch = "wasm32"))]
use nym_validator_client::nyxd::traits::DkgQueryClient;

#[cfg(target_arch = "wasm32")]
use nym_bandwidth_controller::wasm_mockups::DkgQueryClient;

#[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
pub mod non_wasm_helpers;

pub mod helpers;

#[derive(Clone)]
pub struct ClientInput {
    pub connection_command_sender: ConnectionCommandSender,
    pub input_sender: InputMessageSender,
}

impl ClientInput {
    pub async fn send(
        &self,
        message: InputMessage,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<InputMessage>> {
        self.input_sender.send(message).await
    }
}

#[derive(Clone)]
pub struct ClientOutput {
    pub received_buffer_request_sender: ReceivedBufferRequestSender,
}

impl ClientOutput {
    pub fn register_receiver(
        &mut self,
    ) -> Result<mpsc::UnboundedReceiver<Vec<ReconstructedMessage>>, ClientCoreError> {
        let (reconstructed_sender, reconstructed_receiver) = mpsc::unbounded();

        self.received_buffer_request_sender
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                reconstructed_sender,
            ))
            .map_err(|_| ClientCoreError::FailedToRegisterReceiver)?;

        Ok(reconstructed_receiver)
    }
}

#[derive(Clone, Debug)]
pub struct ClientState {
    pub shared_lane_queue_lengths: LaneQueueLengths,
    pub reply_controller_sender: ReplyControllerSender,
    pub topology_accessor: TopologyAccessor,
}

pub enum ClientInputStatus {
    AwaitingProducer { client_input: ClientInput },
    Connected,
}

impl ClientInputStatus {
    pub fn register_producer(&mut self) -> ClientInput {
        match std::mem::replace(self, ClientInputStatus::Connected) {
            ClientInputStatus::AwaitingProducer { client_input } => client_input,
            ClientInputStatus::Connected => panic!("producer was already registered before"),
        }
    }
}

pub enum ClientOutputStatus {
    AwaitingConsumer { client_output: ClientOutput },
    Connected,
}

impl ClientOutputStatus {
    pub fn register_consumer(&mut self) -> ClientOutput {
        match std::mem::replace(self, ClientOutputStatus::Connected) {
            ClientOutputStatus::AwaitingConsumer { client_output } => client_output,
            ClientOutputStatus::Connected => panic!("consumer was already registered before"),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CredentialsToggle {
    Enabled,
    Disabled,
}

impl CredentialsToggle {
    pub fn is_enabled(&self) -> bool {
        self == &CredentialsToggle::Enabled
    }

    pub fn is_disabled(&self) -> bool {
        self == &CredentialsToggle::Disabled
    }
}

impl From<bool> for CredentialsToggle {
    fn from(value: bool) -> Self {
        if value {
            CredentialsToggle::Enabled
        } else {
            CredentialsToggle::Disabled
        }
    }
}

pub struct BaseClientBuilder<'a, B, C, St: Storage> {
    // due to wasm limitations I had to split it like this : (
    gateway_config: &'a GatewayEndpointConfig,
    debug_config: &'a DebugConfig,
    disabled_credentials: bool,
    nym_api_endpoints: Vec<Url>,
    reply_storage_backend: B,

    custom_topology_provider: Option<Box<dyn TopologyProvider>>,
    bandwidth_controller: Option<BandwidthController<C, St>>,
    key_manager: KeyManager,
}

impl<'a, B, C, St> BaseClientBuilder<'a, B, C, St>
where
    B: ReplyStorageBackend + Send + Sync + 'static,
    C: DkgQueryClient + Sync + Send + 'static,
    St: Storage + 'static,
{
    pub fn new_from_base_config<T>(
        base_config: &'a Config<T>,
        key_manager: KeyManager,
        bandwidth_controller: Option<BandwidthController<C, St>>,
        reply_storage_backend: B,
    ) -> BaseClientBuilder<'a, B, C, St> {
        BaseClientBuilder {
            gateway_config: base_config.get_gateway_endpoint_config(),
            debug_config: base_config.get_debug_config(),
            disabled_credentials: base_config.get_disabled_credentials_mode(),
            nym_api_endpoints: base_config.get_nym_api_endpoints(),
            bandwidth_controller,
            reply_storage_backend,
            key_manager,
            custom_topology_provider: None,
        }
    }

    pub fn new(
        gateway_config: &'a GatewayEndpointConfig,
        debug_config: &'a DebugConfig,
        key_manager: KeyManager,
        bandwidth_controller: Option<BandwidthController<C, St>>,
        reply_storage_backend: B,
        credentials_toggle: CredentialsToggle,
        nym_api_endpoints: Vec<Url>,
    ) -> BaseClientBuilder<'a, B, C, St> {
        BaseClientBuilder {
            gateway_config,
            debug_config,
            disabled_credentials: credentials_toggle.is_disabled(),
            nym_api_endpoints,
            reply_storage_backend,
            custom_topology_provider: None,
            bandwidth_controller,
            key_manager,
        }
    }

    pub fn with_topology_provider(mut self, provider: Box<dyn TopologyProvider>) -> Self {
        self.custom_topology_provider = Some(provider);
        self
    }

    pub fn as_mix_recipient(&self) -> Recipient {
        Recipient::new(
            *self.key_manager.identity_keypair().public_key(),
            *self.key_manager.encryption_keypair().public_key(),
            // TODO: below only works under assumption that gateway address == gateway id
            // (which currently is true)
            NodeIdentity::from_base58_string(&self.gateway_config.gateway_id).unwrap(),
        )
    }

    // future constantly pumping loop cover traffic at some specified average rate
    // the pumped traffic goes to the MixTrafficController
    fn start_cover_traffic_stream(
        debug_config: &DebugConfig,
        ack_key: Arc<AckKey>,
        self_address: Recipient,
        topology_accessor: TopologyAccessor,
        mix_tx: BatchMixMessageSender,
        shutdown: TaskClient,
    ) {
        info!("Starting loop cover traffic stream...");

        let stream = LoopCoverTrafficStream::new(
            ack_key,
            debug_config.acknowledgements.average_ack_delay,
            mix_tx,
            self_address,
            topology_accessor,
            debug_config.traffic,
            debug_config.cover_traffic,
        );

        stream.start_with_shutdown(shutdown);
    }

    #[allow(clippy::too_many_arguments)]
    fn start_real_traffic_controller(
        controller_config: real_messages_control::Config,
        topology_accessor: TopologyAccessor,
        ack_receiver: AcknowledgementReceiver,
        input_receiver: InputMessageReceiver,
        mix_sender: BatchMixMessageSender,
        reply_storage: CombinedReplyStorage,
        reply_controller_sender: ReplyControllerSender,
        reply_controller_receiver: ReplyControllerReceiver,
        lane_queue_lengths: LaneQueueLengths,
        client_connection_rx: ConnectionCommandReceiver,
        shutdown: TaskClient,
    ) {
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
        local_encryption_keypair: Arc<encryption::KeyPair>,
        query_receiver: ReceivedBufferRequestReceiver,
        mixnet_receiver: MixnetMessageReceiver,
        reply_key_storage: SentReplyKeys,
        reply_controller_sender: ReplyControllerSender,
        shutdown: TaskClient,
    ) {
        info!("Starting received messages buffer controller...");
        let controller: ReceivedMessagesBufferController<SphinxMessageReceiver> =
            ReceivedMessagesBufferController::new(
                local_encryption_keypair,
                query_receiver,
                mixnet_receiver,
                reply_key_storage,
                reply_controller_sender,
            );
        controller.start_with_shutdown(shutdown)
    }

    async fn start_gateway_client(
        &mut self,
        mixnet_message_sender: MixnetMessageSender,
        ack_sender: AcknowledgementSender,
        shutdown: TaskClient,
    ) -> Result<GatewayClient<C, St>, ClientCoreError> {
        let gateway_id = self.gateway_config.gateway_id.clone();
        if gateway_id.is_empty() {
            return Err(ClientCoreError::GatewayIdUnknown);
        }
        let gateway_address = self.gateway_config.gateway_listener.clone();
        if gateway_address.is_empty() {
            return Err(ClientCoreError::GatewayAddressUnknown);
        }

        let gateway_identity = identity::PublicKey::from_base58_string(gateway_id)
            .map_err(ClientCoreError::UnableToCreatePublicKeyFromGatewayId)?;

        // disgusting wasm workaround since there's no key persistence there (nor `client init`)
        let shared_key = if self.key_manager.is_gateway_key_set() {
            Some(self.key_manager.gateway_shared_key())
        } else {
            log::info!("Gateway key not set! Will proceed anyway.");
            None
        };

        let mut gateway_client = GatewayClient::new(
            gateway_address,
            self.key_manager.identity_keypair(),
            gateway_identity,
            shared_key,
            mixnet_message_sender,
            ack_sender,
            self.debug_config
                .gateway_connection
                .gateway_response_timeout,
            self.bandwidth_controller.take(),
            shutdown,
        );

        gateway_client.set_disabled_credentials_mode(self.disabled_credentials);

        gateway_client
            .authenticate_and_start()
            .await
            .tap_err(|err| {
                log::error!("Could not authenticate and start up the gateway connection - {err}")
            })?;
        Ok(gateway_client)
    }

    fn setup_topology_provider(
        custom_provider: Option<Box<dyn TopologyProvider>>,
        nym_api_urls: Vec<Url>,
    ) -> Box<dyn TopologyProvider> {
        // if no custom provider was ... provided ..., create one using nym-api
        custom_provider.unwrap_or_else(|| {
            Box::new(NymApiTopologyProvider::new(
                nym_api_urls,
                env!("CARGO_PKG_VERSION").to_string(),
            ))
        })
    }

    // future responsible for periodically polling directory server and updating
    // the current global view of topology
    async fn start_topology_refresher(
        topology_provider: Box<dyn TopologyProvider>,
        topology_config: config::Topology,
        topology_accessor: TopologyAccessor,
        mut shutdown: TaskClient,
    ) -> Result<(), ClientCoreError> {
        let topology_refresher_config =
            TopologyRefresherConfig::new(topology_config.topology_refresh_rate);

        let mut topology_refresher = TopologyRefresher::new(
            topology_refresher_config,
            topology_accessor,
            topology_provider,
        );
        // before returning, block entire runtime to refresh the current network view so that any
        // components depending on topology would see a non-empty view
        info!("Obtaining initial network topology");
        topology_refresher.try_refresh().await;

        if let Err(err) = topology_refresher.ensure_topology_is_routable().await {
            log::error!(
                "The current network topology seem to be insufficient to route any packets through \
                - check if enough nodes and a gateway are online - source: {err}"
            );
            return Err(ClientCoreError::InsufficientNetworkTopology(err));
        }

        if topology_config.disable_refreshing {
            // if we're not spawning the refresher, don't cause shutdown immediately
            info!("The topology refesher is not going to be started");
            shutdown.mark_as_success();
        } else {
            // don't spawn the refresher if we don't want to be refreshing the topology.
            // only use the initial values obtained
            info!("Starting topology refresher...");
            topology_refresher.start_with_shutdown(shutdown);
        }

        Ok(())
    }

    // controller for sending packets to mixnet (either real traffic or cover traffic)
    // TODO: if we want to send control messages to gateway_client, this CAN'T take the ownership
    // over it. Perhaps GatewayClient needs to be thread-shareable or have some channel for
    // requests?
    fn start_mix_traffic_controller(
        gateway_client: GatewayClient<C, St>,
        shutdown: TaskClient,
    ) -> BatchMixMessageSender {
        info!("Starting mix traffic controller...");
        let (mix_traffic_controller, mix_tx) = MixTrafficController::new(gateway_client);
        mix_traffic_controller.start_with_shutdown(shutdown);
        mix_tx
    }

    async fn setup_persistent_reply_storage(
        backend: B,
        shutdown: TaskClient,
    ) -> Result<CombinedReplyStorage, ClientCoreError>
    where
        <B as ReplyStorageBackend>::StorageError: Sync + Send,
    {
        if backend.is_active() {
            log::trace!("Setup persistent reply storage");
            let persistent_storage = PersistentReplyStorage::new(backend);
            let mem_store = persistent_storage
                .load_state_from_backend()
                .await
                .map_err(|err| ClientCoreError::SurbStorageError {
                    source: Box::new(err),
                })?;

            let store_clone = mem_store.clone();
            spawn_future(async move {
                persistent_storage
                    .flush_on_shutdown(store_clone, shutdown)
                    .await
            });

            Ok(mem_store)
        } else {
            log::trace!("Setup inactive reply storage");
            Ok(backend
                .get_inactive_storage()
                .map_err(|err| ClientCoreError::SurbStorageError {
                    source: Box::new(err),
                })?)
        }
    }

    pub async fn start_base(mut self) -> Result<BaseClient, ClientCoreError>
    where
        <B as ReplyStorageBackend>::StorageError: Sync + Send,
    {
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

        // Shutdown notifier for signalling tasks to stop
        let task_manager = TaskManager::default();

        // channels responsible for dealing with reply-related fun
        let (reply_controller_sender, reply_controller_receiver) =
            reply_controller::requests::new_control_channels();

        let self_address = self.as_mix_recipient();

        // the components are started in very specific order. Unless you know what you are doing,
        // do not change that.
        let gateway_client = self
            .start_gateway_client(mixnet_messages_sender, ack_sender, task_manager.subscribe())
            .await?;

        let reply_storage = Self::setup_persistent_reply_storage(
            self.reply_storage_backend,
            task_manager.subscribe(),
        )
        .await?;

        let topology_provider = Self::setup_topology_provider(
            self.custom_topology_provider.take(),
            self.nym_api_endpoints,
        );
        Self::start_topology_refresher(
            topology_provider,
            self.debug_config.topology,
            shared_topology_accessor.clone(),
            task_manager.subscribe(),
        )
        .await?;

        Self::start_received_messages_buffer_controller(
            self.key_manager.encryption_keypair(),
            received_buffer_request_receiver,
            mixnet_messages_receiver,
            reply_storage.key_storage(),
            reply_controller_sender.clone(),
            task_manager.subscribe(),
        );

        // The message_sender is the transmitter for any component generating sphinx packets
        // that are to be sent to the mixnet. They are used by cover traffic stream and real
        // traffic stream.
        // The MixTrafficController then sends the actual traffic
        let message_sender =
            Self::start_mix_traffic_controller(gateway_client, task_manager.subscribe());

        // Channels that the websocket listener can use to signal downstream to the real traffic
        // controller that connections are closed.
        let (client_connection_tx, client_connection_rx) = mpsc::unbounded();

        // Shared queue length data. Published by the `OutQueueController` in the client, and used
        // primarily to throttle incoming connections (e.g socks5 for attached network-requesters)
        let shared_lane_queue_lengths = LaneQueueLengths::new();

        let controller_config = real_messages_control::Config::new(
            self.debug_config,
            self.key_manager.ack_key(),
            self_address,
        );

        Self::start_real_traffic_controller(
            controller_config,
            shared_topology_accessor.clone(),
            ack_receiver,
            input_receiver,
            message_sender.clone(),
            reply_storage,
            reply_controller_sender.clone(),
            reply_controller_receiver,
            shared_lane_queue_lengths.clone(),
            client_connection_rx,
            task_manager.subscribe(),
        );

        if !self
            .debug_config
            .cover_traffic
            .disable_loop_cover_traffic_stream
        {
            Self::start_cover_traffic_stream(
                self.debug_config,
                self.key_manager.ack_key(),
                self_address,
                shared_topology_accessor.clone(),
                message_sender,
                task_manager.subscribe(),
            );
        }

        debug!("Core client startup finished!");
        debug!("The address of this client is: {self_address}");

        Ok(BaseClient {
            client_input: ClientInputStatus::AwaitingProducer {
                client_input: ClientInput {
                    connection_command_sender: client_connection_tx,
                    input_sender,
                },
            },
            client_output: ClientOutputStatus::AwaitingConsumer {
                client_output: ClientOutput {
                    received_buffer_request_sender,
                },
            },
            client_state: ClientState {
                shared_lane_queue_lengths,
                reply_controller_sender,
                topology_accessor: shared_topology_accessor,
            },
            task_manager,
        })
    }
}

pub struct BaseClient {
    pub client_input: ClientInputStatus,
    pub client_output: ClientOutputStatus,
    pub client_state: ClientState,

    pub task_manager: TaskManager,
}
