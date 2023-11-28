#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
    time::Duration,
};

use error::IpPacketRouterError;
use futures::{channel::oneshot, StreamExt};
use nym_client_core::{
    client::mix_traffic::transceiver::GatewayTransceiver,
    config::disk_persistence::CommonClientPaths, HardcodedTopologyProvider, TopologyProvider,
};
use nym_ip_packet_requests::{
    DynamicConnectFailureReason, IpPacketRequest, IpPacketRequestData, IpPacketResponse,
    StaticConnectFailureReason,
};
use nym_sdk::{
    mixnet::{InputMessage, MixnetMessageSender, Recipient},
    NymNetworkDetails,
};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::{connections::TransmissionLane, TaskClient, TaskHandle};
use request_filter::RequestFilter;

use crate::config::BaseClientConfig;

pub use crate::config::Config;

pub mod config;
pub mod error;
mod generate_new_ip;
mod request_filter;

// The interface used to route traffic
pub const TUN_BASE_NAME: &str = "nymtun";
pub const TUN_DEVICE_ADDRESS: &str = "10.0.0.1";
pub const TUN_DEVICE_NETMASK: &str = "255.255.255.0";

const DISCONNECT_TIMER_INTERVAL: Duration = Duration::from_secs(10);
const CLIENT_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(5 * 60);

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
        let mixnet_client = create_mixnet_client(
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
            base_name: TUN_BASE_NAME.to_string(),
            ip: TUN_DEVICE_ADDRESS.parse().unwrap(),
            netmask: TUN_DEVICE_NETMASK.parse().unwrap(),
        };
        let (tun, tun_task_tx, tun_task_response_rx) = nym_tun::tun_device::TunDevice::new(
            nym_tun::tun_device::RoutingMode::new_nat(),
            config,
        );
        tun.start();

        let request_filter = request_filter::RequestFilter::new(&self.config).await?;
        request_filter.start_update_tasks().await;

        let ip_packet_router_service = IpPacketRouter {
            _config: self.config,
            request_filter: request_filter.clone(),
            tun_task_tx,
            tun_task_response_rx: Some(tun_task_response_rx),
            mixnet_client,
            task_handle,
            // connected_clients: Default::default(),
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

#[cfg_attr(not(target_os = "linux"), allow(unused))]
struct IpPacketRouter {
    _config: Config,
    request_filter: request_filter::RequestFilter,
    tun_task_tx: nym_tun::tun_task_channel::TunTaskTx,
    tun_task_response_rx: Option<nym_tun::tun_task_channel::TunTaskResponseRx>,
    mixnet_client: nym_sdk::mixnet::MixnetClient,
    task_handle: TaskHandle,
    // connected_clients: HashMap<IpAddr, ConnectedClient>,
}

struct ConnectedClient {
    nym_address: Recipient,
    last_activity: std::time::Instant,
}

#[cfg_attr(not(target_os = "linux"), allow(unused))]
impl IpPacketRouter {
    async fn on_static_connect_request(
        &mut self,
        connect_request: nym_ip_packet_requests::StaticConnectRequest,
        connected_clients: &Arc<std::sync::Mutex<HashMap<IpAddr, ConnectedClient>>>,
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
        // let is_ip_taken = self.connected_clients.contains_key(&requested_ip);
        let is_ip_taken = connected_clients
            .lock()
            .unwrap()
            .contains_key(&requested_ip);

        // Check that the nym address isn't already registered
        let is_nym_address_taken = connected_clients
            .lock()
            .unwrap()
            .values()
            .any(|client| client.nym_address == reply_to);

        match (is_ip_taken, is_nym_address_taken) {
            (true, true) => {
                log::info!("Connecting an already connected client");
                // Update the last activity time for the client
                if let Some(client) = connected_clients.lock().unwrap().get_mut(&requested_ip) {
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
                connected_clients.lock().unwrap().insert(
                    requested_ip,
                    ConnectedClient {
                        nym_address: reply_to,
                        last_activity: std::time::Instant::now(),
                    },
                );
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
        connected_clients: &Arc<std::sync::Mutex<HashMap<IpAddr, ConnectedClient>>>,
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
        let existing_ip = connected_clients.lock().unwrap().iter().find_map(|(ip, client)| {
            if client.nym_address == reply_to {
                Some(*ip)
            } else {
                None
            }
        });

        if let Some(existing_ip) = existing_ip {
            log::info!("Found existing client for nym address");
            // Update the last activity time for the client
            if let Some(client) = connected_clients.lock().unwrap().get_mut(&existing_ip) {
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

        let Some(new_ip) = generate_new_ip::find_new_ip(&connected_clients.lock().unwrap()) else {
            log::info!("No available IP address");
            return Ok(Some(IpPacketResponse::new_dynamic_connect_failure(
                request_id,
                reply_to,
                DynamicConnectFailureReason::NoAvailableIp,
            )));
        };

        connected_clients.lock().unwrap().insert(
            new_ip,
            ConnectedClient {
                nym_address: reply_to,
                last_activity: std::time::Instant::now(),
            },
        );
        Ok(Some(IpPacketResponse::new_dynamic_connect_success(
            request_id, reply_to, new_ip,
        )))
    }

    async fn on_data_request(
        &mut self,
        data_request: nym_ip_packet_requests::DataRequest,
        connected_clients: &Arc<std::sync::Mutex<HashMap<IpAddr, ConnectedClient>>>,
    ) -> Result<Option<IpPacketResponse>, IpPacketRouterError> {
        log::info!("Received data request");

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
        if let Some(client) = connected_clients.lock().unwrap().get_mut(&src_addr) {
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
        // TODO: consider just removing the tag
        let tag = 0;
        self.tun_task_tx
            .try_send((tag, data_request.ip_packet.into()))
            .map_err(|err| IpPacketRouterError::FailedToSendPacketToTun { source: err })?;

        Ok(None)
    }
    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
        connected_clients: &Arc<std::sync::Mutex<HashMap<IpAddr, ConnectedClient>>>,
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
                self.on_static_connect_request(connect_request, connected_clients)
                    .await
            }
            IpPacketRequestData::DynamicConnect(connect_request) => {
                self.on_dynamic_connect_request(connect_request, connected_clients).await
            }
            IpPacketRequestData::Data(data_request) => self.on_data_request(data_request, connected_clients).await,
        }
    }

    async fn run(mut self) -> Result<(), IpPacketRouterError> {
        let mut task_client = self.task_handle.fork("main_loop");
        let mut disconnect_timer = tokio::time::interval(DISCONNECT_TIMER_INTERVAL);

        let mixnet_client_sender = self.mixnet_client.split_sender();
        let mixnet_client_sender_clone = mixnet_client_sender.clone();

        let connected_clients = Arc::new(std::sync::Mutex::new(
            HashMap::<IpAddr, ConnectedClient>::new(),
        ));
        let connected_clients_clone = connected_clients.clone();

        let tun_task_response_rx = self.tun_task_response_rx.take();

        // Spawn TUN listener
        tokio::spawn(async move {
            let mut tun_task_response_rx = tun_task_response_rx.unwrap();
            loop {
                tokio::select! {
                    packet = tun_task_response_rx.recv() => {
                        if let Some((_tag, packet)) = packet {
                            // TODO: skip full parsing since we only need dst_addr
                            let Ok(ParsedPacket {
                                packet_type: _,
                                src_addr: _,
                                dst_addr,
                                dst: _,
                            }) = parse_packet(&packet) else {
                                log::warn!("Failed to parse packet");
                                continue;
                            };

                            let recipient = connected_clients_clone.lock().unwrap().get(&dst_addr).map(|c| c.nym_address);

                            if let Some(recipient) = recipient {
                                let lane = TransmissionLane::General;
                                let packet_type = None;
                                let response_packet = IpPacketResponse::new_ip_packet(packet.into()).to_bytes();
                                let Ok(response_packet) = response_packet else {
                                    log::error!("Failed to serialize response packet");
                                    continue;
                                };
                                let input_message = InputMessage::new_regular(recipient, response_packet, lane, packet_type);

                                if let Err(err) = mixnet_client_sender_clone.send(input_message).await {
                                    log::error!("IpPacketRouter [main loop]: failed to send packet to mixnet: {err}");
                                };
                            } else {
                                log::error!("IpPacketRouter [main loop]: no nym-address recipient for packet");
                            }
                        } else {
                            log::trace!("IpPacketRouter [main loop]: stopping since channel closed");
                            break;
                        }
                    }
                }
            }
        });

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("IpPacketRouter [main loop]: received shutdown");
                },
                _ = disconnect_timer.tick() => {
                    let now = std::time::Instant::now();
                    let inactive_clients: Vec<IpAddr> = connected_clients.lock().unwrap().iter()
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
                        connected_clients.lock().unwrap().remove(&ip);
                    }
                },
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        match self.on_reconstructed_message(msg, &connected_clients).await {
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
                                if let Err(err) = mixnet_client_sender.send(input_message).await {
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
        log::info!("IpPacketRouter: stopping");
        Ok(())
    }
}

struct ParsedPacket<'a> {
    packet_type: &'a str,
    src_addr: IpAddr,
    dst_addr: IpAddr,
    dst: Option<SocketAddr>,
}

fn parse_packet(packet: &[u8]) -> Result<ParsedPacket, IpPacketRouterError> {
    let headers = etherparse::SlicedPacket::from_ip(packet).map_err(|err| {
        log::warn!("Unable to parse incoming data as IP packet: {err}");
        IpPacketRouterError::PacketParseFailed { source: err }
    })?;

    let (packet_type, dst_port) = match headers.transport {
        Some(etherparse::TransportSlice::Udp(header)) => ("ipv4", Some(header.destination_port())),
        Some(etherparse::TransportSlice::Tcp(header)) => ("ipv6", Some(header.destination_port())),
        Some(etherparse::TransportSlice::Icmpv4(_)) => ("icmpv4", None),
        Some(etherparse::TransportSlice::Icmpv6(_)) => ("icmpv6", None),
        Some(etherparse::TransportSlice::Unknown(_)) => ("unknown", None),
        None => {
            log::warn!("Received packet missing transport header");
            return Err(IpPacketRouterError::PacketMissingTransportHeader);
        }
    };

    let (src_addr, dst_addr, dst) = match headers.ip {
        Some(etherparse::InternetSlice::Ipv4(ipv4_header, _)) => {
            let src_addr: IpAddr = ipv4_header.source_addr().into();
            let dst_addr: IpAddr = ipv4_header.destination_addr().into();
            let dst = dst_port.map(|port| SocketAddr::new(dst_addr, port));
            (src_addr, dst_addr, dst)
        }
        Some(etherparse::InternetSlice::Ipv6(ipv6_header, _)) => {
            let src_addr: IpAddr = ipv6_header.source_addr().into();
            let dst_addr: IpAddr = ipv6_header.destination_addr().into();
            let dst = dst_port.map(|port| SocketAddr::new(dst_addr, port));
            (src_addr, dst_addr, dst)
        }
        None => {
            log::warn!("Received packet missing IP header");
            return Err(IpPacketRouterError::PacketMissingIpHeader);
        }
    };
    Ok(ParsedPacket {
        packet_type,
        src_addr,
        dst_addr,
        dst,
    })
}

// Helper function to create the mixnet client.
// This is NOT in the SDK since we don't want to expose any of the client-core config types.
// We could however consider moving it to a crate in common in the future.
// TODO: refactor this function and its arguments
#[allow(unused)]
async fn create_mixnet_client(
    config: &BaseClientConfig,
    shutdown: TaskClient,
    custom_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    wait_for_gateway: bool,
    paths: &CommonClientPaths,
) -> Result<nym_sdk::mixnet::MixnetClient, IpPacketRouterError> {
    let debug_config = config.debug;

    let storage_paths = nym_sdk::mixnet::StoragePaths::from(paths.clone());

    let mut client_builder =
        nym_sdk::mixnet::MixnetClientBuilder::new_with_default_storage(storage_paths)
            .await
            .map_err(|err| IpPacketRouterError::FailedToSetupMixnetClient { source: err })?
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
        .map_err(|err| IpPacketRouterError::FailedToSetupMixnetClient { source: err })?;

    mixnet_client
        .connect_to_mixnet()
        .await
        .map_err(|err| IpPacketRouterError::FailedToConnectToMixnet { source: err })
}
