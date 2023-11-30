#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::Path,
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
#[cfg(target_os = "linux")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
        let (tun_reader, tun_writer) =
            tokio::io::split(nym_tun::tun_device::TunDevice::new_device_only(config));

        // Channel used by the IpPacketRouter to signal connected and disconnected clients to the
        // TunListener
        let (connected_client_tx, connected_client_rx) = tokio::sync::mpsc::unbounded_channel();

        let tun_listener = TunListener {
            tun_reader,
            mixnet_client_sender: mixnet_client.split_sender(),
            task_client: task_handle.get_handle(),
            connected_clients: Default::default(),
            connected_client_rx,
        };
        tun_listener.start();

        let request_filter = request_filter::RequestFilter::new(&self.config).await?;
        request_filter.start_update_tasks().await;

        let ip_packet_router_service = IpPacketRouter {
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
struct IpPacketRouter {
    _config: Config,
    request_filter: request_filter::RequestFilter,
    tun_writer: tokio::io::WriteHalf<tokio_tun::Tun>,
    mixnet_client: nym_sdk::mixnet::MixnetClient,
    task_handle: TaskHandle,

    connected_clients: HashMap<IpAddr, ConnectedClient>,
    connected_client_tx: tokio::sync::mpsc::UnboundedSender<ConnectedClientEvent>,
}

struct ConnectedClient {
    nym_address: Recipient,
    last_activity: std::time::Instant,
}

#[cfg(target_os = "linux")]
impl IpPacketRouter {
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
        log::info!("IpPacketRouter: stopping");
        Ok(())
    }
}

// Reads packet from TUN and writes to mixnet client
#[cfg(target_os = "linux")]
struct TunListener {
    tun_reader: tokio::io::ReadHalf<tokio_tun::Tun>,
    mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    task_client: TaskClient,

    // A mirror of the one in IpPacketRouter
    connected_clients: HashMap<IpAddr, ConnectedClient>,
    connected_client_rx: tokio::sync::mpsc::UnboundedReceiver<ConnectedClientEvent>,
}

#[cfg(target_os = "linux")]
impl TunListener {
    async fn run(mut self) -> Result<(), IpPacketRouterError> {
        let mut buf = [0u8; 65535];
        while !self.task_client.is_shutdown() {
            tokio::select! {
                event = self.connected_client_rx.recv() => match event {
                    Some(ConnectedClientEvent::Connect(ip, nym_addr)) => {
                        log::trace!("Connect client: {ip}");
                        self.connected_clients.insert(ip, ConnectedClient {
                            nym_address: *nym_addr,
                            last_activity: std::time::Instant::now(),
                        });
                    },
                    Some(ConnectedClientEvent::Disconnect(ip)) => {
                        log::trace!("Disconnect client: {ip}");
                        self.connected_clients.remove(&ip);
                    },
                    None => {},
                },
                len = self.tun_reader.read(&mut buf) => match len {
                    Ok(len) => {
                        let Some(dst_addr) = parse_dst_addr(&buf[..len]) else {
                            log::warn!("Failed to parse packet");
                            continue;
                        };

                        let recipient = self.connected_clients.get(&dst_addr).map(|c| c.nym_address);

                        if let Some(recipient) = recipient {
                            let lane = TransmissionLane::General;
                            let packet_type = None;
                            let packet = buf[..len].to_vec();
                            let response_packet = IpPacketResponse::new_ip_packet(packet.into()).to_bytes();
                            let Ok(response_packet) = response_packet else {
                                log::error!("Failed to serialize response packet");
                                continue;
                            };
                            let input_message = InputMessage::new_regular(recipient, response_packet, lane, packet_type);

                            if let Err(err) = self.mixnet_client_sender.send(input_message).await {
                                log::error!("TunListener: failed to send packet to mixnet: {err}");
                            };
                        } else {
                            log::info!("No registered nym-address for packet - dropping");
                        }
                    },
                    Err(err) => {
                        log::warn!("iface: read error: {err}");
                        // break;
                    }
                }
            }
        }
        log::info!("TunListener: stopping");
        Ok(())
    }

    fn start(self) {
        tokio::spawn(async move {
            if let Err(err) = self.run().await {
                log::error!("tun listener router has failed: {err}")
            }
        });
    }
}

enum ConnectedClientEvent {
    Disconnect(IpAddr),
    Connect(IpAddr, Box<Recipient>),
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
        Some(etherparse::TransportSlice::Udp(header)) => ("udp", Some(header.destination_port())),
        Some(etherparse::TransportSlice::Tcp(header)) => ("tcp", Some(header.destination_port())),
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

// Constants for IPv4 and IPv6 headers
const IPV4_DEST_ADDR_START: usize = 16;
const IPV4_DEST_ADDR_LEN: usize = 4;
const IPV6_DEST_ADDR_START: usize = 24;
const IPV6_DEST_ADDR_LEN: usize = 16;

// Only parse the destination address, for when we don't need the other stuff
fn parse_dst_addr(packet: &[u8]) -> Option<IpAddr> {
    let version = packet.first().map(|v| v >> 4)?;
    match version {
        4 => {
            // IPv4
            let addr_end = IPV4_DEST_ADDR_START + IPV4_DEST_ADDR_LEN;
            let addr_array: [u8; IPV4_DEST_ADDR_LEN] = packet
                .get(IPV4_DEST_ADDR_START..addr_end)?
                .try_into()
                .ok()?;
            Some(IpAddr::V4(Ipv4Addr::from(addr_array)))
        }
        6 => {
            // IPv6
            let addr_end = IPV6_DEST_ADDR_START + IPV6_DEST_ADDR_LEN;
            let addr_array: [u8; IPV6_DEST_ADDR_LEN] = packet
                .get(IPV6_DEST_ADDR_START..addr_end)?
                .try_into()
                .ok()?;
            Some(IpAddr::V6(Ipv6Addr::from(addr_array)))
        }
        _ => None, // Unknown IP version
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_destination_from_ip_packet() {
        // Create packet
        let builder =
            etherparse::PacketBuilder::ipv4([192, 168, 1, 1], [192, 168, 1, 2], 20).udp(21, 1234);
        let payload = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut packet = Vec::<u8>::with_capacity(builder.size(payload.len()));
        builder.write(&mut packet, &payload).unwrap();

        let dst_addr = parse_dst_addr(&packet).unwrap();
        assert_eq!(dst_addr, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)));
    }
}
