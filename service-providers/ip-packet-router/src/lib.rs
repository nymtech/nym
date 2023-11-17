#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::{
    net::{IpAddr, SocketAddr},
    path::Path,
};

use error::IpPacketRouterError;
use futures::{channel::oneshot, StreamExt};
use nym_client_core::{
    client::mix_traffic::transceiver::GatewayTransceiver,
    config::disk_persistence::CommonClientPaths, HardcodedTopologyProvider, TopologyProvider,
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
mod request_filter;

// The interface used to route traffic
pub const TUN_BASE_NAME: &str = "nymtun";
pub const TUN_DEVICE_ADDRESS: &str = "10.0.0.1";
pub const TUN_DEVICE_NETMASK: &str = "255.255.255.0";

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
        let config = nym_wireguard::tun_device::TunDeviceConfig {
            base_name: TUN_BASE_NAME.to_string(),
            ip: TUN_DEVICE_ADDRESS.parse().unwrap(),
            netmask: TUN_DEVICE_NETMASK.parse().unwrap(),
        };
        let (tun, tun_task_tx, tun_task_response_rx) = nym_wireguard::tun_device::TunDevice::new(
            nym_wireguard::tun_device::RoutingMode::new_nat(),
            config,
        );
        tun.start();

        let request_filter = request_filter::RequestFilter::new(&self.config).await?;
        request_filter.start_update_tasks().await;

        let ip_packet_router_service = IpPacketRouter {
            _config: self.config,
            request_filter: request_filter.clone(),
            tun_task_tx,
            tun_task_response_rx,
            mixnet_client,
            task_handle,
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

#[allow(unused)]
struct IpPacketRouter {
    _config: Config,
    request_filter: request_filter::RequestFilter,
    tun_task_tx: nym_wireguard::tun_task_channel::TunTaskTx,
    tun_task_response_rx: nym_wireguard::tun_task_channel::TunTaskResponseRx,
    mixnet_client: nym_sdk::mixnet::MixnetClient,
    task_handle: TaskHandle,
}

#[allow(unused)]
impl IpPacketRouter {
    async fn run(mut self) -> Result<(), IpPacketRouterError> {
        let mut task_client = self.task_handle.fork("main_loop");

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("IpPacketRouter [main loop]: received shutdown");
                },
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        if let Err(err) = self.on_message(msg).await {
                            log::error!("Error handling mixnet message: {err}");
                        };
                    } else {
                        log::trace!("IpPacketRouter [main loop]: stopping since channel closed");
                        break;
                    };
                },
                packet = self.tun_task_response_rx.recv() => {
                    if let Some((_tag, packet)) = packet {
                        // Read recipient from env variable NYM_CLIENT_ADDR which is a base58
                        // string of the nym-address of the client that the packet should be
                        // sent back to.
                        //
                        // In the near future we will let the client expose it's nym-address
                        // directly, and after that, provide SURBS
                        let recipient = std::env::var("NYM_CLIENT_ADDR").ok().and_then(|addr| {
                            Recipient::try_from_base58_string(addr).ok()
                        });

                        if let Some(recipient) = recipient {
                            let lane = TransmissionLane::General;
                            let packet_type = None;
                            let input_message = InputMessage::new_regular(recipient, packet, lane, packet_type);

                            if let Err(err) = self.mixnet_client.send(input_message).await {
                                log::error!("IpPacketRouter [main loop]: failed to send packet to mixnet: {err}");
                            };
                        } else {
                            log::error!("NYM_CLIENT_ADDR not set or invalid");
                        }
                    } else {
                        log::trace!("IpPacketRouter [main loop]: stopping since channel closed");
                        break;
                    }
                }

            }
        }
        log::info!("IpPacketRouter: stopping");
        Ok(())
    }

    async fn on_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> Result<(), IpPacketRouterError> {
        log::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );

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
        } = parse_packet(&reconstructed.message)?;

        let dst_str = dst.map_or(dst_addr.to_string(), |dst| dst.to_string());
        log::info!("Received packet: {packet_type}: {src_addr} -> {dst_str}");

        // Filter check
        if let Some(dst) = dst {
            if !self.request_filter.check_address(&dst).await {
                log::warn!("Failed filter check: {dst}");
                // TODO: we could consider sending back a response here
                return Err(IpPacketRouterError::AddressFailedFilterCheck { addr: dst });
            }
        } else {
            // TODO: are we always allowing packets without port numbers?
            log::warn!(
                "Ignoring filter check for packet without port number! (TODO: is this correct?)"
            );
        }

        // TODO: set the tag correctly. Can we just reuse sender_tag?
        let peer_tag = 0;
        self.tun_task_tx
            .try_send((peer_tag, reconstructed.message))
            .map_err(|err| IpPacketRouterError::FailedToSendPacketToTun { source: err })?;

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
        Some(etherparse::TransportSlice::Icmpv4(_)) => ("icpv4", None),
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
