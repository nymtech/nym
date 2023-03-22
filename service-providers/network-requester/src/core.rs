// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::allowed_hosts;
use crate::allowed_hosts::OutboundRequestFilter;
use crate::config::Config;
use crate::error::NetworkRequesterError;
use crate::reply::MixnetMessage;
use crate::statistics::ServiceStatisticsCollector;
use crate::{reply, socks5};
use async_trait::async_trait;
use futures::channel::mpsc;
use log::warn;
use nym_bin_common::build_information::BinaryBuildInformation;
use nym_service_providers_common::interface::{
    BinaryInformation, ProviderInterfaceVersion, Request, RequestVersion,
};
use nym_service_providers_common::ServiceProvider;
use nym_socks5_proxy_helpers::connection_controller::{
    Controller, ControllerCommand, ControllerSender,
};
use nym_socks5_proxy_helpers::proxy_runner::{MixProxyReader, MixProxySender};
use nym_socks5_requests::{
    ConnectRequest, ConnectionId, NetworkData, SendRequest, Socks5ProtocolVersion,
    Socks5ProviderRequest, Socks5Request, Socks5RequestContent, Socks5Response,
};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_statistics_common::collector::StatisticsSender;
use nym_task::connections::LaneQueueLengths;
use nym_task::{TaskClient, TaskManager};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

// Since it's an atomic, it's safe to be kept static and shared across threads
static ACTIVE_PROXIES: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn new_legacy_request_version() -> RequestVersion<Socks5Request> {
    RequestVersion {
        provider_interface: ProviderInterfaceVersion::Legacy,
        provider_protocol: Socks5ProtocolVersion::Legacy,
    }
}

// TODO: 'Builder' is not the most appropriate name here, but I needed
// ... something ...
pub struct NRServiceProviderBuilder {
    config: Config,
    outbound_request_filter: OutboundRequestFilter,
    open_proxy: bool,
    enable_statistics: bool,
    stats_provider_addr: Option<Recipient>,
}

struct NRServiceProvider {
    outbound_request_filter: OutboundRequestFilter,
    open_proxy: bool,
    mixnet_client: nym_sdk::mixnet::MixnetClient,

    controller_sender: ControllerSender,
    mix_input_sender: MixProxySender<MixnetMessage>,
    //shared_lane_queue_lengths: LaneQueueLengths,
    stats_collector: Option<ServiceStatisticsCollector>,
    shutdown: TaskManager,
}

#[async_trait]
impl ServiceProvider<Socks5Request> for NRServiceProvider {
    type ServiceProviderError = NetworkRequesterError;

    async fn on_request(
        &mut self,
        sender: Option<AnonymousSenderTag>,
        request: Request<Socks5Request>,
    ) -> Result<(), Self::ServiceProviderError> {
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
            build_information: BinaryBuildInformation::new(env!("CARGO_PKG_VERSION")).to_owned(),
        })
    }

    async fn handle_provider_data_request(
        &mut self,
        sender: Option<AnonymousSenderTag>,
        request: Socks5Request,
        interface_version: ProviderInterfaceVersion,
    ) -> Result<Option<Socks5Response>, Self::ServiceProviderError> {
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
                        .get(&req.conn_id)
                    {
                        stats_collector
                            .request_stats_data
                            .write()
                            .await
                            .processed(remote_addr, req.data.len() as u32);
                    }
                }
                self.handle_proxy_send(req)
            }
        }

        Ok(None)
    }
}

impl NRServiceProviderBuilder {
    pub async fn new(
        config: Config,
        open_proxy: bool,
        enable_statistics: bool,
        stats_provider_addr: Option<Recipient>,
    ) -> NRServiceProviderBuilder {
        let standard_hosts = allowed_hosts::fetch_standard_allowed_list().await;

        log::info!("Standard allowed hosts: {:?}", standard_hosts);

        let allowed_hosts = allowed_hosts::HostsStore::new(
            allowed_hosts::HostsStore::default_base_dir(),
            PathBuf::from("allowed.list"),
            Some(standard_hosts),
        );

        let unknown_hosts = allowed_hosts::HostsStore::new(
            allowed_hosts::HostsStore::default_base_dir(),
            PathBuf::from("unknown.list"),
            None,
        );

        let outbound_request_filter = OutboundRequestFilter::new(allowed_hosts, unknown_hosts);
        NRServiceProviderBuilder {
            config,
            outbound_request_filter,
            open_proxy,
            enable_statistics,
            stats_provider_addr,
        }
    }

    /// Start all subsystems
    pub async fn run_service_provider(self) -> Result<(), NetworkRequesterError> {
        // Connect to the mixnet
        let mixnet_client = create_mixnet_client(self.config.get_base()).await?;

        // channels responsible for managing messages that are to be sent to the mix network. The receiver is
        // going to be used by `mixnet_response_listener`
        let (mix_input_sender, mix_input_receiver) = tokio::sync::mpsc::channel::<MixnetMessage>(1);

        // Used to notify tasks to shutdown. Not all tasks fully supports this (yet).
        let shutdown = nym_task::TaskManager::default();

        // Controller for managing all active connections.
        let (mut active_connections_controller, controller_sender) = Controller::new(
            mixnet_client.connection_command_sender(),
            shutdown.subscribe(),
        );

        tokio::spawn(async move {
            active_connections_controller.run().await;
        });

        let stats_collector = if self.enable_statistics {
            let stats_collector =
                ServiceStatisticsCollector::new(self.stats_provider_addr, mix_input_sender.clone())
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
        let mixnet_client_sender = mixnet_client.sender();
        let self_address = *mixnet_client.nym_address();

        // start the listener for mix messages
        tokio::spawn(async move {
            NRServiceProvider::mixnet_response_listener(
                mixnet_client_sender,
                mix_input_receiver,
                stats_collector_clone,
            )
            .await;
        });

        let service_provider = NRServiceProvider {
            outbound_request_filter: self.outbound_request_filter,
            open_proxy: self.open_proxy,
            mixnet_client,
            controller_sender,
            mix_input_sender,
            //shared_lane_queue_lengths: mixnet_client.shared_lane_queue_lengths(),
            stats_collector,
            shutdown,
        };

        log::info!("The address of this client is: {}", self_address);
        log::info!("All systems go. Press CTRL-C to stop the server.");
        service_provider.run().await
    }
}

impl NRServiceProvider {
    async fn run(mut self) -> Result<(), NetworkRequesterError> {
        // TODO: incorporate graceful shutdowns
        while let Some(reconstructed_messages) = self.mixnet_client.wait_for_messages().await {
            for reconstructed in reconstructed_messages {
                let sender = reconstructed.sender_tag;
                let request = match Socks5ProviderRequest::try_from_bytes(&reconstructed.message) {
                    Ok(req) => req,
                    Err(err) => {
                        // TODO: or should it even be further lowered to debug/trace?
                        log::warn!("Failed to deserialize received message: {err}");
                        continue;
                    }
                };

                if let Err(err) = self.on_request(sender, request).await {
                    // TODO: again, should it be a warning?
                    // we should also probably log some information regarding the origin of the request
                    // so that it would be easier to debug it
                    log::warn!("failed to resolve the received request: {err}");
                }
            }
        }

        log::error!("Network requester exited unexpectedly");
        Ok(())
    }

    /// Listens for any messages from `mix_reader` that should be written back to the mix network
    /// via the `websocket_writer`.
    async fn mixnet_response_listener(
        mut mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
        mut mix_input_reader: MixProxyReader<MixnetMessage>,
        stats_collector: Option<ServiceStatisticsCollector>,
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

                        let response_message = msg.into_input_message();
                        mixnet_client_sender.send_input_message(response_message).await;
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
        controller_sender: ControllerSender,
        mix_input_sender: MixProxySender<MixnetMessage>,
        lane_queue_lengths: LaneQueueLengths,
        shutdown: TaskClient,
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
                log::error!(
                    "error while connecting to {:?} ! - {:?}",
                    remote_addr.clone(),
                    err
                );

                // inform the remote that the connection is closed before it even was established
                let mixnet_message = MixnetMessage::new_network_data_response(
                    return_address,
                    remote_version,
                    connection_id,
                    NetworkData::new_closed_empty(connection_id),
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
            "Starting proxy for {} (currently there are {} proxies being handled)",
            remote_addr,
            old_count + 1
        );

        // run the proxy on the connection
        conn.run_proxy(
            remote_version,
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
            "Proxy for {} is finished  (currently there are {} proxies being handled)",
            remote_addr,
            old_count - 1
        );
    }

    async fn handle_proxy_connect(
        &mut self,
        remote_version: RequestVersion<Socks5Request>,
        sender_tag: Option<AnonymousSenderTag>,
        connect_req: Box<ConnectRequest>,
    ) {
        let Some(return_address) = reply::MixnetAddress::new(connect_req.return_address, sender_tag) else {
            log::warn!(
                "attempted to start connection with no way of returning data back to the sender"
            );
            return;
        };

        let remote_addr = connect_req.remote_addr;
        let conn_id = connect_req.conn_id;

        if !self.open_proxy && !self.outbound_request_filter.check(&remote_addr) {
            let log_msg = format!("Domain {remote_addr:?} failed filter check");
            log::info!("{}", log_msg);
            let msg = MixnetMessage::new_connection_error(
                return_address,
                remote_version,
                conn_id,
                log_msg,
            );
            self.mix_input_sender
                .send(msg)
                .await
                .expect("InputMessageReceiver has stopped receiving!");
            return;
        }

        let controller_sender_clone = self.controller_sender.clone();
        let mix_input_sender_clone = self.mix_input_sender.clone();
        let lane_queue_lengths_clone = self.mixnet_client.shared_lane_queue_lengths();
        let shutdown = self.shutdown.subscribe();

        // and start the proxy for this connection
        tokio::spawn(async move {
            Self::start_proxy(
                remote_version,
                conn_id,
                remote_addr,
                return_address,
                controller_sender_clone,
                mix_input_sender_clone,
                lane_queue_lengths_clone,
                shutdown,
            )
            .await
        });
    }

    fn handle_proxy_send(&mut self, req: SendRequest) {
        self.controller_sender.unbounded_send(req.into()).unwrap()
    }
}

// Helper function to create the mixnet client.
// This is NOT in the SDK since we don't want to expose any of the client-core config types.
// We could however consider moving it to a crate in common in the future.
async fn create_mixnet_client<T>(
    config: &client_core::config::Config<T>,
) -> Result<nym_sdk::mixnet::MixnetClient, NetworkRequesterError> {
    let nym_api_endpoints = config.get_nym_api_endpoints();
    let debug_config = config.get_debug_config().clone();

    let mixnet_config = nym_sdk::mixnet::Config {
        user_chosen_gateway: None,
        nym_api_endpoints,
        debug_config,
    };

    let storage_paths = nym_sdk::mixnet::StoragePaths::from(config);

    let mixnet_client = nym_sdk::mixnet::MixnetClientBuilder::new()
        .config(mixnet_config)
        .enable_storage(storage_paths)
        .gateway_config(config.get_gateway_endpoint_config().clone())
        .build::<nym_sdk::mixnet::ReplyStorage>()
        .await
        .map_err(|err| NetworkRequesterError::FailedToSetupMixnetClient { source: err })?;

    mixnet_client
        .connect_to_mixnet()
        .await
        .map_err(|err| NetworkRequesterError::FailedToConnectToMixnet { source: err })
}
