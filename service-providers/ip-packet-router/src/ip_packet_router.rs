#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
#![cfg_attr(not(target_os = "linux"), allow(unused_imports))]

use std::{collections::HashMap, net::IpAddr, path::Path};

use futures::{channel::oneshot, StreamExt};
use nym_client_core::{
    client::mix_traffic::transceiver::GatewayTransceiver, HardcodedTopologyProvider,
    TopologyProvider,
};
use nym_ip_packet_requests::{
    DynamicConnectFailureReason, IpPacketRequest, IpPacketRequestData, IpPacketResponse,
    StaticConnectFailureReason,
};
use nym_sdk::mixnet::{InputMessage, MixnetMessageSender, Recipient};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::{connections::TransmissionLane, TaskClient, TaskHandle};
#[cfg(target_os = "linux")]
use tokio::io::AsyncWriteExt;

use crate::{
    constants::{CLIENT_INACTIVITY_TIMEOUT, DISCONNECT_TIMER_INTERVAL},
    error::IpPacketRouterError,
    request_filter::{self, RequestFilter},
    util::generate_new_ip,
    util::parse_ip::{parse_packet, ParsedPacket},
    Config,
};

pub struct OnStartData {
    // to add more fields as required
    pub address: Recipient,

    pub request_filter: RequestFilter,
}

impl OnStartData {
    pub fn new(address: Recipient, request_filter: RequestFilter) -> Self {
        Self {
            address,
            request_filter,
        }
    }
}

pub struct IpPacketRouterBuilder {
    #[allow(unused)]
    config: Config,
    wait_for_gateway: bool,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    custom_gateway_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    shutdown: Option<TaskClient>,
    on_start: Option<oneshot::Sender<OnStartData>>,
}

impl IpPacketRouterBuilder {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            wait_for_gateway: false,
            custom_topology_provider: None,
            custom_gateway_transceiver: None,
            shutdown: None,
            on_start: None,
        }
    }

    #[must_use]
    pub fn with_shutdown(mut self, shutdown: TaskClient) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    #[must_use]
    pub fn with_custom_gateway_transceiver(
        mut self,
        gateway_transceiver: Box<dyn GatewayTransceiver + Send + Sync>,
    ) -> Self {
        self.custom_gateway_transceiver = Some(gateway_transceiver);
        self
    }

    #[must_use]
    pub fn with_wait_for_gateway(mut self, wait_for_gateway: bool) -> Self {
        self.wait_for_gateway = wait_for_gateway;
        self
    }

    #[must_use]
    pub fn with_on_start(mut self, on_start: oneshot::Sender<OnStartData>) -> Self {
        self.on_start = Some(on_start);
        self
    }

    #[must_use]
    pub fn with_custom_topology_provider(
        mut self,
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Self {
        self.custom_topology_provider = Some(topology_provider);
        self
    }

    pub fn with_stored_topology<P: AsRef<Path>>(
        mut self,
        file: P,
    ) -> Result<Self, IpPacketRouterError> {
        self.custom_topology_provider =
            Some(Box::new(HardcodedTopologyProvider::new_from_file(file)?));
        Ok(self)
    }

    #[cfg(not(target_os = "linux"))]
    pub async fn run_service_provider(self) -> Result<(), IpPacketRouterError> {
        todo!("service provider is not yet supported on this platform")
    }

    #[cfg(target_os = "linux")]
    pub async fn run_service_provider(self) -> Result<(), IpPacketRouterError> {
        // Used to notify tasks to shutdown. Not all tasks fully supports this (yet).
        let task_handle: TaskHandle = self.shutdown.map(Into::into).unwrap_or_default();

        // Connect to the mixnet
        let mixnet_client = crate::mixnet_client::create_mixnet_client(
            &self.config.base,
            task_handle.get_handle().named("nym_sdk::MixnetClient"),
            self.custom_gateway_transceiver,
            self.custom_topology_provider,
            self.wait_for_gateway,
            &self.config.storage_paths.common_paths,
        )
        .await?;

        let self_address = *mixnet_client.nym_address();

        // Create the TUN device that we interact with the rest of the world with
        let config = nym_tun::tun_device::TunDeviceConfig {
            base_name: crate::constants::TUN_BASE_NAME.to_string(),
            ip: crate::constants::TUN_DEVICE_ADDRESS.parse().unwrap(),
            netmask: crate::constants::TUN_DEVICE_NETMASK.parse().unwrap(),
        };
        let (tun_reader, tun_writer) =
            tokio::io::split(nym_tun::tun_device::TunDevice::new_device_only(config));

        // Channel used by the IpPacketRouter to signal connected and disconnected clients to the
        // TunListener
        let (connected_client_tx, connected_client_rx) = tokio::sync::mpsc::unbounded_channel();

        let tun_listener = crate::tun_listener::TunListener {
            tun_reader,
            mixnet_client_sender: mixnet_client.split_sender(),
            task_client: task_handle.get_handle(),
            connected_clients: Default::default(),
            connected_client_rx,
        };
        tun_listener.start();

        let request_filter = request_filter::RequestFilter::new(&self.config).await?;
        request_filter.start_update_tasks().await;

        let ip_packet_router_service = MixnetListener {
            _config: self.config,
            request_filter: request_filter.clone(),
            tun_writer,
            mixnet_client,
            task_handle,
            connected_clients: Default::default(),
            connected_client_tx,
        };

        log::info!("The address of this client is: {self_address}");
        log::info!("All systems go. Press CTRL-C to stop the server.");

        if let Some(on_start) = self.on_start {
            if on_start
                .send(OnStartData::new(self_address, request_filter))
                .is_err()
            {
                // the parent has dropped the channel before receiving the response
                return Err(IpPacketRouterError::DisconnectedParent);
            }
        }

        ip_packet_router_service.run().await
    }
}

#[cfg(target_os = "linux")]
struct MixnetListener {
    _config: Config,
    request_filter: request_filter::RequestFilter,
    tun_writer: tokio::io::WriteHalf<tokio_tun::Tun>,
    mixnet_client: nym_sdk::mixnet::MixnetClient,
    task_handle: TaskHandle,

    connected_clients: HashMap<IpAddr, ConnectedClient>,
    connected_client_tx: tokio::sync::mpsc::UnboundedSender<ConnectedClientEvent>,
}

pub(crate) struct ConnectedClient {
    pub(crate) nym_address: Recipient,
    pub(crate) last_activity: std::time::Instant,
}

#[cfg(target_os = "linux")]
impl MixnetListener {
    async fn on_static_connect_request(
        &mut self,
        connect_request: nym_ip_packet_requests::StaticConnectRequest,
    ) -> Result<Option<IpPacketResponse>, IpPacketRouterError> {
        log::info!(
            "Received static connect request from {sender_address}",
            sender_address = connect_request.reply_to
        );

        let request_id = connect_request.request_id;
        let requested_ip = connect_request.ip;
        let reply_to = connect_request.reply_to;
        // TODO: ignoring reply_to_hops and reply_to_avg_mix_delays for now

        // Check that the IP is available in the set of connected clients
        let is_ip_taken = self.connected_clients.contains_key(&requested_ip);

        // Check that the nym address isn't already registered
        let is_nym_address_taken = self
            .connected_clients
            .values()
            .any(|client| client.nym_address == reply_to);

        match (is_ip_taken, is_nym_address_taken) {
            (true, true) => {
                log::info!("Connecting an already connected client");
                // Update the last activity time for the client
                if let Some(client) = self.connected_clients.get_mut(&requested_ip) {
                    client.last_activity = std::time::Instant::now();
                } else {
                    log::error!("Failed to update last activity time for client");
                }
                Ok(Some(IpPacketResponse::new_static_connect_success(
                    request_id, reply_to,
                )))
            }
            (false, false) => {
                log::info!("Connecting a new client");
                self.connected_clients.insert(
                    requested_ip,
                    ConnectedClient {
                        nym_address: reply_to,
                        last_activity: std::time::Instant::now(),
                    },
                );
                self.connected_client_tx
                    .send(ConnectedClientEvent::Connect(
                        requested_ip,
                        Box::new(reply_to),
                    ))
                    .unwrap();
                Ok(Some(IpPacketResponse::new_static_connect_success(
                    request_id, reply_to,
                )))
            }
            (true, false) => {
                log::info!("Requested IP is not available");
                Ok(Some(IpPacketResponse::new_static_connect_failure(
                    request_id,
                    reply_to,
                    StaticConnectFailureReason::RequestedIpAlreadyInUse,
                )))
            }
            (false, true) => {
                log::info!("Nym address is already registered");
                Ok(Some(IpPacketResponse::new_static_connect_failure(
                    request_id,
                    reply_to,
                    StaticConnectFailureReason::RequestedNymAddressAlreadyInUse,
                )))
            }
        }
    }

    async fn on_dynamic_connect_request(
        &mut self,
        connect_request: nym_ip_packet_requests::DynamicConnectRequest,
    ) -> Result<Option<IpPacketResponse>, IpPacketRouterError> {
        log::info!(
            "Received dynamic connect request from {sender_address}",
            sender_address = connect_request.reply_to
        );

        let request_id = connect_request.request_id;
        let reply_to = connect_request.reply_to;
        // TODO: ignoring reply_to_hops and reply_to_avg_mix_delays for now

        // Check if it's the same client connecting again, then we just reuse the same IP
        // TODO: this is problematic. Until we sign connect requests this means you can spam people
        // with return traffic
        let existing_ip = self.connected_clients.iter().find_map(|(ip, client)| {
            if client.nym_address == reply_to {
                Some(*ip)
            } else {
                None
            }
        });

        if let Some(existing_ip) = existing_ip {
            log::info!("Found existing client for nym address");
            // Update the last activity time for the client
            if let Some(client) = self.connected_clients.get_mut(&existing_ip) {
                client.last_activity = std::time::Instant::now();
            } else {
                log::error!("Failed to update last activity time for client");
            }
            return Ok(Some(IpPacketResponse::new_dynamic_connect_success(
                request_id,
                reply_to,
                existing_ip,
            )));
        }

        let Some(new_ip) = generate_new_ip::find_new_ip(&self.connected_clients) else {
            log::info!("No available IP address");
            return Ok(Some(IpPacketResponse::new_dynamic_connect_failure(
                request_id,
                reply_to,
                DynamicConnectFailureReason::NoAvailableIp,
            )));
        };

        self.connected_clients.insert(
            new_ip,
            ConnectedClient {
                nym_address: reply_to,
                last_activity: std::time::Instant::now(),
            },
        );
        self.connected_client_tx
            .send(ConnectedClientEvent::Connect(new_ip, Box::new(reply_to)))
            .unwrap();
        Ok(Some(IpPacketResponse::new_dynamic_connect_success(
            request_id, reply_to, new_ip,
        )))
    }

    async fn on_data_request(
        &mut self,
        data_request: nym_ip_packet_requests::DataRequest,
    ) -> Result<Option<IpPacketResponse>, IpPacketRouterError> {
        log::trace!("Received data request");

        // We don't forward packets that we are not able to parse. BUT, there might be a good
        // reason to still forward them.
        //
        // For example, if we are running in a mode where we are only supposed to forward
        // packets to a specific destination, we might want to forward them anyway.
        //
        // TODO: look into this
        let ParsedPacket {
            packet_type,
            src_addr,
            dst_addr,
            dst,
        } = parse_packet(&data_request.ip_packet)?;

        let dst_str = dst.map_or(dst_addr.to_string(), |dst| dst.to_string());
        log::info!("Received packet: {packet_type}: {src_addr} -> {dst_str}");

        // Check if there is a connected client for this src_addr. If there is, update the last activity time
        // for the client. If there isn't, drop the packet.
        if let Some(client) = self.connected_clients.get_mut(&src_addr) {
            client.last_activity = std::time::Instant::now();
        } else {
            log::info!("Dropping packet: no connected client for {src_addr}");
            return Ok(None);
        }

        // Filter check
        if let Some(dst) = dst {
            if !self.request_filter.check_address(&dst).await {
                log::warn!("Failed filter check: {dst}");
                // TODO: we could consider sending back a response here
                return Err(IpPacketRouterError::AddressFailedFilterCheck { addr: dst });
            }
        } else {
            // TODO: we should also filter packets without port number
            log::warn!("Ignoring filter check for packet without port number! TODO!");
        }

        // TODO: consider changing from Vec<u8> to bytes::Bytes?
        let packet = data_request.ip_packet;
        self.tun_writer
            .write_all(&packet)
            .await
            .map_err(|_| IpPacketRouterError::FailedToWritePacketToTun)?;

        Ok(None)
    }
    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> Result<Option<IpPacketResponse>, IpPacketRouterError> {
        log::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );

        // Check version of request
        if let Some(version) = reconstructed.message.first() {
            // The idea is that in the future we can add logic here to parse older versions to stay
            // backwards compatible.
            if *version != nym_ip_packet_requests::CURRENT_VERSION {
                log::warn!("Received packet with invalid version");
                return Err(IpPacketRouterError::InvalidPacketVersion(*version));
            }
        }

        let request = IpPacketRequest::from_reconstructed_message(&reconstructed)
            .map_err(|err| IpPacketRouterError::FailedToDeserializeTaggedPacket { source: err })?;

        match request.data {
            IpPacketRequestData::StaticConnect(connect_request) => {
                self.on_static_connect_request(connect_request).await
            }
            IpPacketRequestData::DynamicConnect(connect_request) => {
                self.on_dynamic_connect_request(connect_request).await
            }
            IpPacketRequestData::Data(data_request) => self.on_data_request(data_request).await,
        }
    }

    async fn run(mut self) -> Result<(), IpPacketRouterError> {
        let mut task_client = self.task_handle.fork("main_loop");
        let mut disconnect_timer = tokio::time::interval(DISCONNECT_TIMER_INTERVAL);

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("IpPacketRouter [main loop]: received shutdown");
                },
                _ = disconnect_timer.tick() => {
                    let now = std::time::Instant::now();
                    let inactive_clients: Vec<IpAddr> = self.connected_clients.iter()
                        .filter_map(|(ip, client)| {
                            if now.duration_since(client.last_activity) > CLIENT_INACTIVITY_TIMEOUT {
                                Some(*ip)
                            } else {
                                None
                            }
                        })
                        .collect();
                    for ip in inactive_clients {
                        log::info!("Disconnect inactive client: {ip}");
                        self.connected_clients.remove(&ip);
                        self.connected_client_tx.send(ConnectedClientEvent::Disconnect(ip)).unwrap();
                    }
                },
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        match self.on_reconstructed_message(msg).await {
                            Ok(Some(response)) => {
                                let Some(recipient) = response.recipient() else {
                                    log::error!("IpPacketRouter [main loop]: failed to get recipient from response");
                                    continue;
                                };
                                let response_packet = response.to_bytes();
                                let Ok(response_packet) = response_packet else {
                                    log::error!("Failed to serialize response packet");
                                    continue;
                                };
                                let lane = TransmissionLane::General;
                                let packet_type = None;
                                let input_message = InputMessage::new_regular(*recipient, response_packet, lane, packet_type);
                                if let Err(err) = self.mixnet_client.send(input_message).await {
                                    log::error!("IpPacketRouter [main loop]: failed to send packet to mixnet: {err}");
                                };
                            },
                            Ok(None) => {
                                continue;
                            },
                            Err(err) => {
                                log::error!("Error handling mixnet message: {err}");
                            }

                        };
                    } else {
                        log::trace!("IpPacketRouter [main loop]: stopping since channel closed");
                        break;
                    };
                },

            }
        }
        log::debug!("IpPacketRouter: stopping");
        Ok(())
    }
}

pub(crate) enum ConnectedClientEvent {
    Disconnect(IpAddr),
    Connect(IpAddr, Box<Recipient>),
}
