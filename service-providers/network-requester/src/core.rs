// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{BaseClientConfig, Config};
use crate::error::NetworkRequesterError;
use crate::reply::MixnetMessage;
use crate::request_filter::RequestFilter;
use crate::statistics::ServiceStatisticsCollector;
use crate::{reply, socks5};
use async_trait::async_trait;
use futures::channel::{mpsc, oneshot};
use futures::stream::StreamExt;
use futures::SinkExt;
use log::{debug, warn};
use nym_bin_common::bin_info_owned;
use nym_client_core::client::mix_traffic::transceiver::GatewayTransceiver;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_client_core::HardcodedTopologyProvider;
use nym_network_defaults::NymNetworkDetails;
use nym_sdk::mixnet::TopologyProvider;
use nym_service_providers_common::interface::{
    BinaryInformation, ProviderInterfaceVersion, Request, RequestVersion,
};
use nym_service_providers_common::ServiceProvider;
use nym_socks5_proxy_helpers::connection_controller::{
    Controller, ControllerCommand, ControllerSender,
};
use nym_socks5_proxy_helpers::proxy_runner::{MixProxyReader, MixProxySender};
use nym_socks5_requests::{
    ConnectRequest, ConnectionId, QueryRequest, QueryResponse, SendRequest, SocketData,
    Socks5ProtocolVersion, Socks5ProviderRequest, Socks5Request, Socks5RequestContent,
    Socks5Response,
};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::params::{PacketSize, PacketType};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_statistics_common::collector::StatisticsSender;
use nym_task::connections::LaneQueueLengths;
use nym_task::manager::TaskHandle;
use nym_task::TaskClient;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio_util::sync::PollSender;

// Since it's an atomic, it's safe to be kept static and shared across threads
static ACTIVE_PROXIES: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn new_legacy_request_version() -> RequestVersion<Socks5Request> {
    RequestVersion {
        provider_interface: ProviderInterfaceVersion::Legacy,
        provider_protocol: Socks5ProtocolVersion::Legacy,
    }
}

#[allow(dead_code)]
pub struct OnStartData {
    // to add more fields as required
    pub address: Recipient,

    pub request_filter: RequestFilter,
}

impl OnStartData {
    fn new(address: Recipient, request_filter: RequestFilter) -> Self {
        Self {
            address,
            request_filter,
        }
    }
}

// TODO: 'Builder' is not the most appropriate name here, but I needed
// ... something ...
pub struct NRServiceProviderBuilder {
    config: Config,

    wait_for_gateway: bool,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    custom_gateway_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    shutdown: Option<TaskClient>,
    on_start: Option<oneshot::Sender<OnStartData>>,
}

pub struct NRServiceProvider {
    config: Config,
    request_filter: RequestFilter,

    mixnet_client: nym_sdk::mixnet::MixnetClient,
    controller_sender: ControllerSender,

    mix_input_sender: MixProxySender<MixnetMessage>,
    stats_collector: Option<ServiceStatisticsCollector>,
    shutdown: TaskHandle,
}

#[async_trait]
impl ServiceProvider<Socks5Request> for NRServiceProvider {
    type ServiceProviderError = NetworkRequesterError;

    async fn on_request(
        &mut self,
        sender: Option<AnonymousSenderTag>,
        request: Request<Socks5Request>,
    ) -> Result<(), Self::ServiceProviderError> {
        // TODO: this should perhaps be parallelised
        log::debug!("on_request {:?}", request);
        if let Some(response) = self.handle_request(sender, request).await? {
            // TODO: this (i.e. `reply::MixnetAddress`) should be incorporated into the actual interface
            if let Some(return_address) = reply::MixnetAddress::new(None, sender) {
                let msg = MixnetMessage::new_provider_response(return_address, 0, response);
                self.mix_input_sender
                    .send(msg)
                    .await
                    .expect("InputMessageReceiver has stopped receiving!");
            } else {
                warn!("currently we can only send generic replies via reply surbs and we haven't got any : (")
            }
        }
        Ok(())
    }

    async fn handle_binary_info_control_request(
        &self,
    ) -> Result<BinaryInformation, Self::ServiceProviderError> {
        Ok(BinaryInformation {
            binary_name: env!("CARGO_PKG_NAME").to_string(),
            build_information: bin_info_owned!(),
        })
    }

    async fn handle_provider_data_request(
        &mut self,
        sender: Option<AnonymousSenderTag>,
        request: Socks5Request,
        interface_version: ProviderInterfaceVersion,
    ) -> Result<Option<Socks5Response>, Self::ServiceProviderError> {
        log::debug!("handle_provider_data_request {:?}", request);

        // TODO: streamline this a bit more
        let request_version = RequestVersion::new(interface_version, request.protocol_version);

        log::debug!(
            "received request of version {:?} (interface) / {:?} (socks5)",
            interface_version,
            request.protocol_version
        );

        match request.content {
            Socks5RequestContent::Connect(req) => {
                if let Some(stats_collector) = &self.stats_collector {
                    stats_collector
                        .connected_services
                        .write()
                        .await
                        .insert(req.conn_id, req.remote_addr.clone());
                }
                self.handle_proxy_connect(request_version, sender, req)
                    .await
            }
            Socks5RequestContent::Send(req) => {
                if let Some(stats_collector) = &self.stats_collector {
                    if let Some(remote_addr) = stats_collector
                        .connected_services
                        .read()
                        .await
                        .get(&req.data.header.connection_id)
                    {
                        stats_collector
                            .request_stats_data
                            .write()
                            .await
                            .processed(remote_addr, req.data.data.len() as u32);
                    }
                }
                self.handle_proxy_send(req)
            }
            Socks5RequestContent::Query(query) => return self.handle_query(query),
        }

        Ok(None)
    }
}

impl NRServiceProviderBuilder {
    pub fn new(config: Config) -> NRServiceProviderBuilder {
        NRServiceProviderBuilder {
            config,
            wait_for_gateway: false,
            custom_topology_provider: None,
            custom_gateway_transceiver: None,
            shutdown: None,
            on_start: None,
        }
    }

    #[must_use]
    // this is a false positive, this method is actually called when used as a library
    // but clippy complains about it when building the binary
    #[allow(unused)]
    pub fn with_shutdown(mut self, shutdown: TaskClient) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    #[must_use]
    // this is a false positive, this method is actually called when used as a library
    // but clippy complains about it when building the binary
    #[allow(unused)]
    pub fn with_custom_gateway_transceiver(
        mut self,
        gateway_transceiver: Box<dyn GatewayTransceiver + Send + Sync>,
    ) -> Self {
        self.custom_gateway_transceiver = Some(gateway_transceiver);
        self
    }

    #[must_use]
    // this is a false positive, this method is actually called when used as a library
    // but clippy complains about it when building the binary
    #[allow(unused)]
    pub fn with_wait_for_gateway(mut self, wait_for_gateway: bool) -> Self {
        self.wait_for_gateway = wait_for_gateway;
        self
    }

    #[must_use]
    // this is a false positive, this method is actually called when used as a library
    // but clippy complains about it when building the binary
    #[allow(unused)]
    pub fn with_on_start(mut self, on_start: oneshot::Sender<OnStartData>) -> Self {
        self.on_start = Some(on_start);
        self
    }

    #[must_use]
    // this is a false positive, this method is actually called when used as a library
    // but clippy complains about it when building the binary
    #[allow(unused)]
    pub fn with_custom_topology_provider(
        mut self,
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Self {
        self.custom_topology_provider = Some(topology_provider);
        self
    }

    // this is a false positive, this method is actually called when used as a library
    // but clippy complains about it when building the binary
    #[allow(unused)]
    pub fn with_stored_topology<P: AsRef<Path>>(
        mut self,
        file: P,
    ) -> Result<Self, NetworkRequesterError> {
        self.custom_topology_provider =
            Some(Box::new(HardcodedTopologyProvider::new_from_file(file)?));
        Ok(self)
    }

    /// Start all subsystems
    pub async fn run_service_provider(self) -> Result<(), NetworkRequesterError> {
        let stats_provider_addr = self
            .config
            .network_requester
            .statistics_recipient
            .as_ref()
            .map(Recipient::try_from_base58_string)
            .transpose()
            .unwrap_or(None);

        // Used to notify tasks to shutdown. Not all tasks fully supports this (yet).
        let shutdown: TaskHandle = self.shutdown.map(Into::into).unwrap_or_default();

        // Connect to the mixnet
        let mixnet_client = create_mixnet_client(
            &self.config.base,
            shutdown.get_handle().named("nym_sdk::MixnetClient"),
            self.custom_gateway_transceiver,
            self.custom_topology_provider,
            self.wait_for_gateway,
            &self.config.storage_paths.common_paths,
        )
        .await?;

        // channels responsible for managing messages that are to be sent to the mix network. The receiver is
        // going to be used by `mixnet_response_listener`
        let (mix_input_sender, mix_input_receiver) = tokio::sync::mpsc::channel::<MixnetMessage>(1);

        let mix_input_sender = PollSender::new(mix_input_sender);

        // Controller for managing all active connections.
        let (mut active_connections_controller, controller_sender) = Controller::new(
            mixnet_client.connection_command_sender(),
            shutdown
                .get_handle()
                .named("nym_socks5_proxy_helpers::connection_controller::Controller"),
        );

        tokio::spawn(async move {
            active_connections_controller.run().await;
        });

        let stats_collector = if self.config.network_requester.enabled_statistics {
            let stats_collector =
                ServiceStatisticsCollector::new(stats_provider_addr, mix_input_sender.clone())
                    .await
                    .expect("Service statistics collector could not be bootstrapped");
            let mut stats_sender = StatisticsSender::new(stats_collector.clone());

            tokio::spawn(async move {
                stats_sender.run().await;
            });
            Some(stats_collector)
        } else {
            None
        };

        let stats_collector_clone = stats_collector.clone();
        let mixnet_client_sender = mixnet_client.split_sender();
        let self_address = *mixnet_client.nym_address();
        let packet_type = self.config.base.debug.traffic.packet_type;

        // start the listener for mix messages
        tokio::spawn(async move {
            NRServiceProvider::mixnet_response_listener(
                mixnet_client_sender,
                mix_input_receiver,
                stats_collector_clone,
                packet_type,
            )
            .await;
        });

        let request_filter = RequestFilter::new(&self.config).await?;

        let mut service_provider = NRServiceProvider {
            config: self.config,
            request_filter: request_filter.clone(),
            mixnet_client,
            controller_sender,
            mix_input_sender,
            stats_collector,
            shutdown,
        };

        log::info!("The address of this client is: {self_address}");
        log::info!("All systems go. Press CTRL-C to stop the server.");

        if let Some(on_start) = self.on_start {
            if on_start
                .send(OnStartData::new(self_address, request_filter))
                .is_err()
            {
                // the parent has dropped the channel before receiving the response
                return Err(NetworkRequesterError::DisconnectedParent);
            }
        }

        service_provider.run().await
    }
}

impl NRServiceProvider {
    async fn run(&mut self) -> Result<(), NetworkRequesterError> {
        let mut shutdown = self.shutdown.fork("main_loop");
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    debug!("NRServiceProvider [main loop]: received shutdown")
                },
                msg = self.mixnet_client.next() => match msg {
                    Some(msg) => self.on_message(msg).await,
                    None => {
                        log::trace!("NRServiceProvider::run: Stopping since channel closed");
                        break;
                    }
                },
            }
        }

        Ok(())
    }

    async fn on_message(&mut self, reconstructed: ReconstructedMessage) {
        let sender = reconstructed.sender_tag;
        let request = match Socks5ProviderRequest::try_from_bytes(&reconstructed.message) {
            Ok(req) => req,
            Err(err) => {
                // TODO: or should it even be further lowered to debug/trace?
                log::warn!("Failed to deserialize received message: {err}");
                return;
            }
        };

        if let Err(err) = self.on_request(sender, request).await {
            // TODO: again, should it be a warning?
            // we should also probably log some information regarding the origin of the request
            // so that it would be easier to debug it
            log::warn!("failed to resolve the received request: {err}");
        }
    }

    /// Listens for any messages from `mix_reader` that should be written back to the mix network
    /// via the `websocket_writer`.
    async fn mixnet_response_listener(
        mut mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
        mut mix_input_reader: MixProxyReader<MixnetMessage>,
        stats_collector: Option<ServiceStatisticsCollector>,
        packet_type: PacketType,
    ) {
        loop {
            tokio::select! {
                socks5_msg = mix_input_reader.recv() => {
                    if let Some(msg) = socks5_msg {
                        if let Some(stats_collector) = stats_collector.as_ref() {
                            if let Some(remote_addr) = stats_collector
                                .connected_services
                                .read()
                                .await
                                .get(&msg.connection_id)
                            {
                                stats_collector
                                    .response_stats_data
                                    .write()
                                    .await
                                    .processed(remote_addr, msg.data_size() as u32);
                            }
                        }

                        let response_message = msg.into_input_message(packet_type);
                        nym_sdk::mixnet::MixnetMessageSender::send(&mut mixnet_client_sender, response_message).await.unwrap();
                    } else {
                        log::error!("Exiting: channel closed!");
                        break;
                    }
                },
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn start_proxy(
        remote_version: RequestVersion<Socks5Request>,
        connection_id: ConnectionId,
        remote_addr: String,
        return_address: reply::MixnetAddress,
        biggest_packet_size: PacketSize,
        controller_sender: ControllerSender,
        mut mix_input_sender: MixProxySender<MixnetMessage>,
        lane_queue_lengths: LaneQueueLengths,
        mut shutdown: TaskClient,
    ) {
        let mut conn = match socks5::tcp::Connection::new(
            connection_id,
            remote_addr.clone(),
            return_address.clone(),
        )
        .await
        {
            Ok(conn) => conn,
            Err(err) => {
                log::error!("error while connecting to {remote_addr}: {err}",);
                shutdown.disarm();

                // inform the remote that the connection is closed before it even was established
                let mixnet_message = MixnetMessage::new_network_data_response(
                    return_address,
                    remote_version,
                    connection_id,
                    SocketData::new(0, connection_id, true, Vec::new()),
                );

                mix_input_sender
                    .send(mixnet_message)
                    .await
                    .expect("InputMessageReceiver has stopped receiving!");

                return;
            }
        };

        // Connect implies it's a fresh connection - register it with our controller
        let (mix_sender, mix_receiver) = mpsc::unbounded();
        controller_sender
            .unbounded_send(ControllerCommand::Insert {
                connection_id,
                connection_sender: mix_sender,
            })
            .unwrap();

        let old_count = ACTIVE_PROXIES.fetch_add(1, Ordering::SeqCst);
        log::info!(
            "Starting proxy for {remote_addr} (currently there are {} proxies being handled)",
            old_count + 1
        );

        // run the proxy on the connection
        conn.run_proxy(
            remote_version,
            biggest_packet_size,
            mix_receiver,
            mix_input_sender,
            lane_queue_lengths,
            shutdown,
        )
        .await;

        // proxy is done - remove the access channel from the controller
        controller_sender
            .unbounded_send(ControllerCommand::Remove { connection_id })
            .unwrap();

        let old_count = ACTIVE_PROXIES.fetch_sub(1, Ordering::SeqCst);
        log::info!(
            "Proxy for {remote_addr} is finished  (currently there are {} proxies being handled)",
            old_count - 1
        );
    }

    async fn handle_proxy_connect(
        &self,
        remote_version: RequestVersion<Socks5Request>,
        sender_tag: Option<AnonymousSenderTag>,
        connect_req: Box<ConnectRequest>,
    ) {
        let Some(return_address) =
            reply::MixnetAddress::new(connect_req.return_address, sender_tag)
        else {
            log::warn!(
                "attempted to start connection with no way of returning data back to the sender"
            );
            return;
        };

        let remote_addr = connect_req.remote_addr;
        let conn_id = connect_req.conn_id;
        let traffic_config = self.config.base.debug.traffic;
        let packet_size = traffic_config
            .secondary_packet_size
            .unwrap_or(traffic_config.primary_packet_size);

        let controller_sender_clone = self.controller_sender.clone();
        let mut mix_input_sender_clone = self.mix_input_sender.clone();
        let lane_queue_lengths_clone = self.mixnet_client.shared_lane_queue_lengths();
        let mut shutdown = self.shutdown.get_handle();

        // we're just cloning the underlying pointer, nothing expensive is happening here
        let request_filter = self.request_filter.clone();

        // at this point move it into the separate task
        // because we might have to resolve the underlying address and it can take some time
        // during which we don't want to block other incoming requests
        tokio::spawn(async move {
            if !request_filter.check_address(&remote_addr).await {
                let log_msg = format!("Domain {remote_addr:?} failed filter check");
                log::info!("{log_msg}");
                let error_msg = MixnetMessage::new_connection_error(
                    return_address,
                    remote_version,
                    conn_id,
                    log_msg,
                );

                mix_input_sender_clone
                    .send(error_msg)
                    .await
                    .expect("InputMessageReceiver has stopped receiving!");
                shutdown.mark_as_success();
                return;
            }

            // if all is good, start the proxy for this connection
            Self::start_proxy(
                remote_version,
                conn_id,
                remote_addr,
                return_address,
                packet_size,
                controller_sender_clone,
                mix_input_sender_clone,
                lane_queue_lengths_clone,
                shutdown,
            )
            .await
        });
    }

    fn handle_proxy_send(&mut self, req: SendRequest) {
        self.controller_sender
            .unbounded_send(ControllerCommand::new_send(req.data))
            .unwrap()
    }

    fn handle_query(
        &self,
        query: QueryRequest,
    ) -> Result<Option<Socks5Response>, NetworkRequesterError> {
        let protocol_version = Socks5ProtocolVersion::default();

        let response = match query {
            QueryRequest::OpenProxy => Socks5Response::new_query(
                protocol_version,
                QueryResponse::OpenProxy(self.config.network_requester.open_proxy),
            ),
            QueryRequest::Description => Socks5Response::new_query(
                protocol_version,
                QueryResponse::Description("Description (placeholder)".to_string()),
            ),
            QueryRequest::ExitPolicy => {
                let exit_policy_filter = self.request_filter.current_exit_policy_filter();
                let response = QueryResponse::ExitPolicy {
                    enabled: true,
                    upstream: exit_policy_filter
                        .upstream()
                        .map(|u| u.to_string())
                        .unwrap_or_default(),
                    policy: Some(exit_policy_filter.policy().clone()),
                };

                Socks5Response::new_query(protocol_version, response)
            }
            _ => {
                Socks5Response::new_query_error(protocol_version, "received unknown query variant")
            }
        };
        Ok(Some(response))
    }
}

// Helper function to create the mixnet client.
// This is NOT in the SDK since we don't want to expose any of the client-core config types.
// We could however consider moving it to a crate in common in the future.
// TODO: refactor this function and its arguments
async fn create_mixnet_client(
    config: &BaseClientConfig,
    shutdown: TaskClient,
    custom_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    wait_for_gateway: bool,
    paths: &CommonClientPaths,
) -> Result<nym_sdk::mixnet::MixnetClient, NetworkRequesterError> {
    let debug_config = config.debug;

    let storage_paths = nym_sdk::mixnet::StoragePaths::from(paths.clone());

    let mut client_builder =
        nym_sdk::mixnet::MixnetClientBuilder::new_with_default_storage(storage_paths)
            .await
            .map_err(|err| NetworkRequesterError::FailedToSetupMixnetClient { source: err })?
            .network_details(NymNetworkDetails::new_from_env())
            .debug_config(debug_config)
            .custom_shutdown(shutdown)
            .with_wait_for_gateway(wait_for_gateway);
    if !config.get_disabled_credentials_mode() {
        client_builder = client_builder.enable_credentials_mode();
    }
    if let Some(gateway_transceiver) = custom_transceiver {
        client_builder = client_builder.custom_gateway_transceiver(gateway_transceiver);
    }
    if let Some(topology_provider) = custom_topology_provider {
        client_builder = client_builder.custom_topology_provider(topology_provider);
    }

    let mixnet_client = client_builder
        .build()
        .map_err(|err| NetworkRequesterError::FailedToSetupMixnetClient { source: err })?;

    mixnet_client
        .connect_to_mixnet()
        .await
        .map_err(|err| NetworkRequesterError::FailedToConnectToMixnet { source: err })
}
