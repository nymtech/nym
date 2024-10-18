// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::metrics::{MetricsController, MetricsSender};
use super::received_buffer::ReceivedBufferMessage;
use super::topology_control::geo_aware_provider::GeoAwareTopologyProvider;
use crate::client::base_client::storage::helpers::store_client_keys;
use crate::client::base_client::storage::MixnetClientStorage;
use crate::client::cover_traffic_stream::LoopCoverTrafficStream;
use crate::client::inbound_messages::{InputMessage, InputMessageReceiver, InputMessageSender};
use crate::client::key_manager::persistence::KeyStore;
use crate::client::key_manager::ClientKeys;
use crate::client::mix_traffic::transceiver::{GatewayReceiver, GatewayTransceiver, RemoteGateway};
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
    nym_api_provider, TopologyAccessor, TopologyRefresher, TopologyRefresherConfig,
};
use crate::config::{Config, DebugConfig};
use crate::error::ClientCoreError;
use crate::init::{
    setup_gateway,
    types::{GatewaySetup, InitialisationResult},
};
use crate::{config, spawn_future};
use futures::channel::mpsc;
use log::*;
use nym_bandwidth_controller::BandwidthController;
use nym_client_core_gateways_storage::{GatewayDetails, GatewaysDetailsStore};
use nym_credential_storage::storage::Storage as CredentialStorage;
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway_client::client::config::GatewayClientConfig;
use nym_gateway_client::{
    AcknowledgementReceiver, GatewayClient, GatewayConfig, MixnetMessageReceiver, PacketRouter,
};
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_sphinx::params::PacketType;
use nym_sphinx::receiver::{ReconstructedMessage, SphinxMessageReceiver};
use nym_task::connections::{ConnectionCommandReceiver, ConnectionCommandSender, LaneQueueLengths};
use nym_task::{TaskClient, TaskHandle};
use nym_topology::provider_trait::TopologyProvider;
use nym_topology::HardcodedTopologyProvider;
use nym_validator_client::{nyxd::contract_traits::DkgQueryClient, UserAgent};
use rand::rngs::OsRng;
use std::fmt::Debug;
use std::os::raw::c_int as RawFd;
use std::path::Path;
use std::sync::Arc;
use url::Url;

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "fs-surb-storage",
    feature = "fs-gateways-storage"
))]
pub mod non_wasm_helpers;

pub mod helpers;
pub mod storage;

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
    pub gateway_connection: GatewayConnection,
}

#[derive(Clone, Copy, Debug)]
pub struct GatewayConnection {
    pub gateway_ws_fd: Option<RawFd>,
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

pub struct BaseClientBuilder<'a, C, S: MixnetClientStorage> {
    config: &'a Config,
    client_store: S,
    dkg_query_client: Option<C>,

    wait_for_gateway: bool,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    custom_gateway_transceiver: Option<Box<dyn GatewayTransceiver + Send>>,
    shutdown: Option<TaskClient>,
    user_agent: Option<UserAgent>,

    setup_method: GatewaySetup,
}

impl<'a, C, S> BaseClientBuilder<'a, C, S>
where
    S: MixnetClientStorage + 'static,
    C: DkgQueryClient + Send + Sync + 'static,
{
    pub fn new(
        base_config: &'a Config,
        client_store: S,
        dkg_query_client: Option<C>,
    ) -> BaseClientBuilder<'a, C, S> {
        BaseClientBuilder {
            config: base_config,
            client_store,
            dkg_query_client,
            wait_for_gateway: false,
            custom_topology_provider: None,
            custom_gateway_transceiver: None,
            shutdown: None,
            user_agent: None,
            setup_method: GatewaySetup::MustLoad { gateway_id: None },
        }
    }

    #[must_use]
    pub fn with_gateway_setup(mut self, setup: GatewaySetup) -> Self {
        self.setup_method = setup;
        self
    }

    #[must_use]
    pub fn with_wait_for_gateway(mut self, wait_for_gateway: bool) -> Self {
        self.wait_for_gateway = wait_for_gateway;
        self
    }

    #[must_use]
    pub fn with_topology_provider(
        mut self,
        provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Self {
        self.custom_topology_provider = Some(provider);
        self
    }

    #[must_use]
    pub fn with_gateway_transceiver(mut self, sender: Box<dyn GatewayTransceiver + Send>) -> Self {
        self.custom_gateway_transceiver = Some(sender);
        self
    }

    #[must_use]
    pub fn with_shutdown(mut self, shutdown: TaskClient) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    #[must_use]
    pub fn with_user_agent(mut self, user_agent: UserAgent) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_stored_topology<P: AsRef<Path>>(
        mut self,
        file: P,
    ) -> Result<Self, ClientCoreError> {
        self.custom_topology_provider =
            Some(Box::new(HardcodedTopologyProvider::new_from_file(file)?));
        Ok(self)
    }

    // note: do **NOT** make this method public as its only valid usage is from within `start_base`
    // because it relies on the crypto keys being already loaded
    fn mix_address(details: &InitialisationResult) -> Recipient {
        details.client_address()
    }

    // future constantly pumping loop cover traffic at some specified average rate
    // the pumped traffic goes to the MixTrafficController
    fn start_cover_traffic_stream(
        debug_config: &DebugConfig,
        ack_key: Arc<AckKey>,
        self_address: Recipient,
        topology_accessor: TopologyAccessor,
        mix_tx: BatchMixMessageSender,
        stats_tx: MetricsSender,
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
            stats_tx,
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
        packet_type: PacketType,
        stats_tx: MetricsSender,
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
            stats_tx,
        )
        .start_with_shutdown(shutdown, packet_type);
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
        metrics_reporter: MetricsSender,
    ) {
        info!("Starting received messages buffer controller...");
        let controller: ReceivedMessagesBufferController<SphinxMessageReceiver> =
            ReceivedMessagesBufferController::new(
                local_encryption_keypair,
                query_receiver,
                mixnet_receiver,
                reply_key_storage,
                reply_controller_sender,
                metrics_reporter,
            );
        controller.start_with_shutdown(shutdown)
    }

    async fn start_gateway_client(
        config: &Config,
        initialisation_result: InitialisationResult,
        bandwidth_controller: Option<BandwidthController<C, S::CredentialStore>>,
        details_store: &S::GatewaysDetailsStore,
        packet_router: PacketRouter,
        shutdown: TaskClient,
    ) -> Result<GatewayClient<C, S::CredentialStore>, ClientCoreError>
    where
        <S::KeyStore as KeyStore>::StorageError: Send + Sync + 'static,
        <S::CredentialStore as CredentialStorage>::StorageError: Send + Sync + 'static,
        <S::GatewaysDetailsStore as GatewaysDetailsStore>::StorageError: Sync + Send,
    {
        let managed_keys = initialisation_result.client_keys;
        let GatewayDetails::Remote(details) = initialisation_result.gateway_registration.details
        else {
            return Err(ClientCoreError::UnexpectedPersistedCustomGatewayDetails);
        };

        let mut gateway_client =
            if let Some(existing_client) = initialisation_result.authenticated_ephemeral_client {
                existing_client.upgrade(packet_router, bandwidth_controller, shutdown)
            } else {
                let cfg = GatewayConfig::new(
                    details.gateway_id,
                    details
                        .gateway_owner_address
                        .as_ref()
                        .map(|o| o.to_string()),
                    details.gateway_listener.to_string(),
                );
                GatewayClient::new(
                    GatewayClientConfig::new_default()
                        .with_disabled_credentials_mode(config.client.disabled_credentials_mode)
                        .with_response_timeout(
                            config.debug.gateway_connection.gateway_response_timeout,
                        ),
                    cfg,
                    managed_keys.identity_keypair(),
                    Some(details.shared_key),
                    packet_router,
                    bandwidth_controller,
                    shutdown,
                )
            };

        let gateway_failure = |err| {
            log::error!("Could not authenticate and start up the gateway connection - {err}");
            ClientCoreError::GatewayClientError {
                gateway_id: details.gateway_id.to_base58_string(),
                source: err,
            }
        };

        // the gateway client startup procedure is slightly more complicated now
        // we need to:
        // - perform handshake (reg or auth)
        // - check for key upgrade
        // - maybe perform another upgrade handshake
        // - check for bandwidth
        // - start background tasks
        let auth_res = gateway_client
            .perform_initial_authentication()
            .await
            .map_err(gateway_failure)?;

        if auth_res.requires_key_upgrade {
            // drop the shared_key arc because we don't need it and we can't hold it for the purposes of upgrade
            drop(auth_res);

            let updated_key = gateway_client
                .upgrade_key_authenticated()
                .await
                .map_err(gateway_failure)?;

            details_store
                .upgrade_stored_remote_gateway_key(gateway_client.gateway_identity(), &updated_key)
                .await.map_err(|err| {
                    error!("failed to store upgraded gateway key! this connection might be forever broken now: {err}");
                    ClientCoreError::GatewaysDetailsStoreError { source: Box::new(err) }
            })?
        }

        gateway_client
            .claim_initial_bandwidth()
            .await
            .map_err(gateway_failure)?;
        gateway_client
            .start_listening_for_mixnet_messages()
            .map_err(gateway_failure)?;

        Ok(gateway_client)
    }

    async fn setup_gateway_transceiver(
        custom_gateway_transceiver: Option<Box<dyn GatewayTransceiver + Send>>,
        config: &Config,
        initialisation_result: InitialisationResult,
        bandwidth_controller: Option<BandwidthController<C, S::CredentialStore>>,
        details_store: &S::GatewaysDetailsStore,
        packet_router: PacketRouter,
        mut shutdown: TaskClient,
    ) -> Result<Box<dyn GatewayTransceiver + Send>, ClientCoreError>
    where
        <S::KeyStore as KeyStore>::StorageError: Send + Sync + 'static,
        <S::CredentialStore as CredentialStorage>::StorageError: Send + Sync + 'static,
        <S::GatewaysDetailsStore as GatewaysDetailsStore>::StorageError: Sync + Send,
    {
        // if we have setup custom gateway sender and persisted details agree with it, return it
        if let Some(mut custom_gateway_transceiver) = custom_gateway_transceiver {
            return if !initialisation_result
                .gateway_registration
                .details
                .is_custom()
            {
                Err(ClientCoreError::CustomGatewaySelectionExpected)
            } else {
                // and make sure to invalidate the task client, so we wouldn't cause premature shutdown
                shutdown.disarm();
                custom_gateway_transceiver.set_packet_router(packet_router)?;
                Ok(custom_gateway_transceiver)
            };
        }

        // otherwise, setup normal gateway client, etc
        let gateway_client = Self::start_gateway_client(
            config,
            initialisation_result,
            bandwidth_controller,
            details_store,
            packet_router,
            shutdown,
        )
        .await?;

        Ok(Box::new(RemoteGateway::new(gateway_client)))
    }

    fn setup_topology_provider(
        custom_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
        config_topology: config::Topology,
        nym_api_urls: Vec<Url>,
        user_agent: Option<UserAgent>,
    ) -> Box<dyn TopologyProvider + Send + Sync> {
        // if no custom provider was ... provided ..., create one using nym-api
        custom_provider.unwrap_or_else(|| match config_topology.topology_structure {
            config::TopologyStructure::NymApi => Box::new(NymApiTopologyProvider::new(
                nym_api_provider::Config {
                    min_mixnode_performance: config_topology.minimum_mixnode_performance,
                    min_gateway_performance: config_topology.minimum_gateway_performance,
                },
                nym_api_urls,
                env!("CARGO_PKG_VERSION").to_string(),
                user_agent,
            )),
            config::TopologyStructure::GeoAware(group_by) => {
                Box::new(GeoAwareTopologyProvider::new(
                    nym_api_urls,
                    env!("CARGO_PKG_VERSION").to_string(),
                    group_by,
                ))
            }
        })
    }

    // future responsible for periodically polling directory server and updating
    // the current global view of topology
    async fn start_topology_refresher(
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
        topology_config: config::Topology,
        topology_accessor: TopologyAccessor,
        local_gateway: &NodeIdentity,
        wait_for_gateway: bool,
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

        let gateway_wait_timeout = if wait_for_gateway {
            Some(topology_config.max_startup_gateway_waiting_period)
        } else {
            None
        };

        if let Err(err) = topology_refresher
            .ensure_contains_gateway(local_gateway)
            .await
        {
            if let Some(waiting_timeout) = gateway_wait_timeout {
                if let Err(err) = topology_refresher
                    .wait_for_gateway(local_gateway, waiting_timeout)
                    .await
                {
                    error!(
                        "the gateway did not come back online within the specified timeout: {err}"
                    );
                    return Err(err.into());
                }
            } else {
                error!("the gateway we're supposedly connected to does not exist. We'll not be able to send any packets to ourselves: {err}");
                return Err(err.into());
            }
        }

        if topology_config.disable_refreshing {
            // if we're not spawning the refresher, don't cause shutdown immediately
            info!("The topology refesher is not going to be started");
            shutdown.disarm();
        } else {
            // don't spawn the refresher if we don't want to be refreshing the topology.
            // only use the initial values obtained
            info!("Starting topology refresher...");
            topology_refresher.start_with_shutdown(shutdown);
        }

        Ok(())
    }

    fn start_packet_statistics_control(shutdown: TaskClient) -> MetricsSender {
        info!("Starting packet statistics control...");
        let (metrics_controller, metrics_reporter) = MetricsController::new();
        metrics_controller.start_with_shutdown(shutdown);
        metrics_reporter
    }

    fn start_mix_traffic_controller(
        gateway_transceiver: Box<dyn GatewayTransceiver + Send>,
        shutdown: TaskClient,
    ) -> BatchMixMessageSender {
        info!("Starting mix traffic controller...");
        let (mix_traffic_controller, mix_tx) = MixTrafficController::new(gateway_transceiver);
        mix_traffic_controller.start_with_shutdown(shutdown);
        mix_tx
    }

    // TODO: rename it as it implies the data is persistent whilst one can use InMemBackend
    async fn setup_persistent_reply_storage(
        backend: S::ReplyStore,
        shutdown: TaskClient,
    ) -> Result<CombinedReplyStorage, ClientCoreError>
    where
        <S::ReplyStore as ReplyStorageBackend>::StorageError: Sync + Send,
        S::ReplyStore: Send + Sync,
    {
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
    }

    async fn initialise_keys_and_gateway(
        setup_method: GatewaySetup,
        key_store: &S::KeyStore,
        details_store: &S::GatewaysDetailsStore,
    ) -> Result<InitialisationResult, ClientCoreError>
    where
        <S::KeyStore as KeyStore>::StorageError: Sync + Send,
        <S::GatewaysDetailsStore as GatewaysDetailsStore>::StorageError: Sync + Send,
    {
        // if client keys do not exist already, create and persist them
        if key_store.load_keys().await.is_err() {
            info!("could not find valid client keys - a new set will be generated");
            let mut rng = OsRng;
            let keys = ClientKeys::generate_new(&mut rng);
            store_client_keys(keys, key_store).await?;
        }

        setup_gateway(setup_method, key_store, details_store).await
    }

    pub async fn start_base(mut self) -> Result<BaseClient, ClientCoreError>
    where
        S::ReplyStore: Send + Sync,
        <S::KeyStore as KeyStore>::StorageError: Send + Sync,
        <S::ReplyStore as ReplyStorageBackend>::StorageError: Sync + Send,
        <S::CredentialStore as CredentialStorage>::StorageError: Send + Sync + 'static,
        <S::GatewaysDetailsStore as GatewaysDetailsStore>::StorageError: Sync + Send,
    {
        info!("Starting nym client");

        // derive (or load) client keys and gateway configuration
        let init_res = Self::initialise_keys_and_gateway(
            self.setup_method,
            self.client_store.key_store(),
            self.client_store.gateway_details_store(),
        )
        .await?;

        let (reply_storage_backend, credential_store, details_store) =
            self.client_store.into_runtime_stores();

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
        let shutdown = self
            .shutdown
            .map(Into::<TaskHandle>::into)
            .unwrap_or_default()
            .name_if_unnamed("BaseNymClient");

        // channels responsible for dealing with reply-related fun
        let (reply_controller_sender, reply_controller_receiver) =
            reply_controller::requests::new_control_channels();

        let self_address = Self::mix_address(&init_res);
        let ack_key = init_res.client_keys.ack_key();
        let encryption_keys = init_res.client_keys.encryption_keypair();
        let identity_keys = init_res.client_keys.identity_keypair();

        // the components are started in very specific order. Unless you know what you are doing,
        // do not change that.
        let bandwidth_controller = self
            .dkg_query_client
            .map(|client| BandwidthController::new(credential_store, client));

        let topology_provider = Self::setup_topology_provider(
            self.custom_topology_provider.take(),
            self.config.debug.topology,
            self.config.get_nym_api_endpoints(),
            self.user_agent.clone(),
        );

        // needs to be started as the first thing to block if required waiting for the gateway
        Self::start_topology_refresher(
            topology_provider,
            self.config.debug.topology,
            shared_topology_accessor.clone(),
            self_address.gateway(),
            self.wait_for_gateway,
            shutdown.fork("topology_refresher"),
        )
        .await?;

        let metrics_reporter =
            Self::start_packet_statistics_control(shutdown.fork("packet_statistics_control"));

        let gateway_packet_router = PacketRouter::new(
            ack_sender,
            mixnet_messages_sender,
            shutdown.get_handle().named("gateway-packet-router"),
        );

        let gateway_transceiver = Self::setup_gateway_transceiver(
            self.custom_gateway_transceiver,
            self.config,
            init_res,
            bandwidth_controller,
            &details_store,
            gateway_packet_router,
            shutdown.fork("gateway_transceiver"),
        )
        .await?;
        let gateway_ws_fd = gateway_transceiver.ws_fd();

        let reply_storage = Self::setup_persistent_reply_storage(
            reply_storage_backend,
            shutdown.fork("persistent_reply_storage"),
        )
        .await?;

        Self::start_received_messages_buffer_controller(
            encryption_keys,
            received_buffer_request_receiver,
            mixnet_messages_receiver,
            reply_storage.key_storage(),
            reply_controller_sender.clone(),
            shutdown.fork("received_messages_buffer"),
            metrics_reporter.clone(),
        );

        // The message_sender is the transmitter for any component generating sphinx packets
        // that are to be sent to the mixnet. They are used by cover traffic stream and real
        // traffic stream.
        // The MixTrafficController then sends the actual traffic
        let message_sender = Self::start_mix_traffic_controller(
            gateway_transceiver,
            shutdown.fork("mix_traffic_controller"),
        );

        // Channels that the websocket listener can use to signal downstream to the real traffic
        // controller that connections are closed.
        let (client_connection_tx, client_connection_rx) = mpsc::unbounded();

        // Shared queue length data. Published by the `OutQueueController` in the client, and used
        // primarily to throttle incoming connections (e.g socks5 for attached network-requesters)
        let shared_lane_queue_lengths = LaneQueueLengths::new();

        let controller_config = real_messages_control::Config::new(
            &self.config.debug,
            Arc::clone(&ack_key),
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
            shutdown.fork("real_traffic_controller"),
            self.config.debug.traffic.packet_type,
            metrics_reporter.clone(),
        );

        if !self
            .config
            .debug
            .cover_traffic
            .disable_loop_cover_traffic_stream
        {
            Self::start_cover_traffic_stream(
                &self.config.debug,
                ack_key,
                self_address,
                shared_topology_accessor.clone(),
                message_sender,
                metrics_reporter,
                shutdown.fork("cover_traffic_stream"),
            );
        }

        debug!("Core client startup finished!");
        debug!("The address of this client is: {self_address}");

        Ok(BaseClient {
            address: self_address,
            identity_keys,
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
                gateway_connection: GatewayConnection { gateway_ws_fd },
            },
            task_handle: shutdown,
        })
    }
}

pub struct BaseClient {
    pub address: Recipient,
    pub identity_keys: Arc<identity::KeyPair>,
    pub client_input: ClientInputStatus,
    pub client_output: ClientOutputStatus,
    pub client_state: ClientState,

    pub task_handle: TaskHandle,
}
