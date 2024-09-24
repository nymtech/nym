use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr};

use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use nym_ip_packet_requests::v7::request::{HealthRequest, PingRequest};
use nym_ip_packet_requests::v7::response::{
    DynamicConnectFailureReason, InfoLevel, InfoResponseReply, StaticConnectFailureReason,
};
use nym_ip_packet_requests::{
    codec::MultiIpPacketCodec,
    v6,
    v7::{
        self,
        request::{
            DataRequest, DisconnectRequest, DynamicConnectRequest, IpPacketRequest,
            IpPacketRequestData, StaticConnectRequest,
        },
        signature::SignedRequest,
    },
    IpPair,
};
use nym_sdk::mixnet::{MixnetMessageSender, Recipient};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;
use tap::TapFallible;
#[cfg(target_os = "linux")]
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tokio_util::codec::Decoder;

use crate::{
    config::Config,
    connected_client_handler,
    constants::{CLIENT_MIXNET_INACTIVITY_TIMEOUT, DISCONNECT_TIMER_INTERVAL},
    error::{IpPacketRouterError, Result},
    request_filter::{self},
    tun_listener,
    util::generate_new_ip,
    util::{
        create_message::create_input_message,
        parse_ip::{parse_packet, ParsedPacket},
    },
};

pub(crate) struct ConnectedClients {
    // The set of connected clients
    clients_ipv4_mapping: HashMap<Ipv4Addr, ConnectedClient>,
    clients_ipv6_mapping: HashMap<Ipv6Addr, ConnectedClient>,

    // Notify the tun listener when a new client connects or disconnects
    tun_listener_connected_client_tx: tokio::sync::mpsc::UnboundedSender<ConnectedClientEvent>,
}

impl ConnectedClients {
    pub(crate) fn new() -> (Self, tun_listener::ConnectedClientsListener) {
        let (connected_client_tx, connected_client_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                clients_ipv4_mapping: Default::default(),
                clients_ipv6_mapping: Default::default(),
                tun_listener_connected_client_tx: connected_client_tx,
            },
            tun_listener::ConnectedClientsListener::new(connected_client_rx),
        )
    }

    fn is_ip_connected(&self, ips: &IpPair) -> bool {
        self.clients_ipv4_mapping.contains_key(&ips.ipv4)
            || self.clients_ipv6_mapping.contains_key(&ips.ipv6)
    }

    fn get_client_from_ip_mut(&mut self, ip: &IpAddr) -> Option<&mut ConnectedClient> {
        match ip {
            IpAddr::V4(ip) => self.clients_ipv4_mapping.get_mut(ip),
            IpAddr::V6(ip) => self.clients_ipv6_mapping.get_mut(ip),
        }
    }

    fn is_nym_address_connected(&self, nym_address: &Recipient) -> bool {
        self.clients_ipv4_mapping
            .values()
            .any(|client| client.nym_address == *nym_address)
    }

    fn lookup_ip_from_nym_address(&self, nym_address: &Recipient) -> Option<IpPair> {
        self.clients_ipv4_mapping
            .iter()
            .find_map(|(ipv4, connected_client)| {
                if connected_client.nym_address == *nym_address {
                    Some(IpPair::new(*ipv4, connected_client.ipv6))
                } else {
                    None
                }
            })
    }

    fn remove_client_with_nym_address(&self, nym_address: &Recipient) {
        // Remove the client from both the ipv4 and ipv6 maps
        let ipv4 = self.clients_ipv4_mapping.iter().find_map(|(ip, client)| {
            if client.nym_address == *nym_address {
                Some(ip)
            } else {
                None
            }
        });
        let client_ipv4 = self.clients_ipv4_mapping.remove(ipv4);

        let ipv6 = self.clients_ipv6_mapping.iter().find_map(|(ip, client)| {
            if client.nym_address == *nym_address {
                Some(ip)
            } else {
                None
            }
        });
        let client_ipv6 = self.clients_ipv6_mapping.remove(ipv6);

        // These two should be the same
        if let Some(client) = client_ipv4 {
            // client.update_activity()
        }
    }

    #[allow(dead_code)]
    fn lookup_client_from_nym_address(&self, nym_address: &Recipient) -> Option<&ConnectedClient> {
        self.clients_ipv4_mapping
            .iter()
            .find_map(|(_, connected_client)| {
                if connected_client.nym_address == *nym_address {
                    Some(connected_client)
                } else {
                    None
                }
            })
    }

    fn connect(
        &mut self,
        ips: IpPair,
        nym_address: Recipient,
        client_version: SupportedClientVersion,
        forward_from_tun_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        close_tx: tokio::sync::oneshot::Sender<()>,
        handle: tokio::task::JoinHandle<()>,
    ) {
        // The map of connected clients that the mixnet listener keeps track of. It monitors
        // activity and disconnects clients that have been inactive for too long.
        let client = ConnectedClient {
            nym_address,
            ipv6: ips.ipv6,
            last_activity: Arc::new(RwLock::new(std::time::Instant::now())),
            client_version,
            _close_tx: Arc::new(CloseTx {
                nym_address,
                inner: Some(close_tx),
            }),
            handle: Arc::new(handle),
        };
        log::info!("Inserting {} and {}", ips.ipv4, ips.ipv6);
        self.clients_ipv4_mapping.insert(ips.ipv4, client.clone());
        self.clients_ipv6_mapping.insert(ips.ipv6, client);
        // Send the connected client info to the tun listener, which will use it to forward packets
        // to the connected client handler.
        self.tun_listener_connected_client_tx
            .send(ConnectedClientEvent::Connect(Box::new(ConnectEvent {
                ips,
                forward_from_tun_tx,
            })))
            .tap_err(|err| {
                log::error!("Failed to send connected client event: {err}");
            })
            .ok();
    }

    async fn update_activity(&mut self, ips: &IpPair) -> Result<()> {
        if let Some(client) = self.clients_ipv4_mapping.get(&ips.ipv4) {
            *client.last_activity.write().await = std::time::Instant::now();
            Ok(())
        } else {
            Err(IpPacketRouterError::FailedToUpdateClientActivity)
        }
    }

    // Identify connected client handlers that have stopped without being told to stop
    fn get_finished_client_handlers(&mut self) -> Vec<(IpPair, Recipient, SupportedClientVersion)> {
        self.clients_ipv4_mapping
            .iter_mut()
            .filter_map(|(ip, connected_client)| {
                if connected_client.handle.is_finished() {
                    Some((
                        IpPair::new(*ip, connected_client.ipv6),
                        connected_client.nym_address,
                        connected_client.client_version,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    async fn get_inactive_clients(&mut self) -> Vec<(IpPair, Recipient, SupportedClientVersion)> {
        let now = std::time::Instant::now();
        let mut ret = vec![];
        for (ip, connected_client) in self.clients_ipv4_mapping.iter() {
            if now.duration_since(*connected_client.last_activity.read().await)
                > CLIENT_MIXNET_INACTIVITY_TIMEOUT
            {
                ret.push((
                    IpPair::new(*ip, connected_client.ipv6),
                    connected_client.nym_address,
                    connected_client.client_version,
                ))
            }
        }
        ret
    }

    fn disconnect(&mut self, ips: &IpPair) {
        self.clients_ipv4_mapping.remove(&ips.ipv4);
        self.clients_ipv6_mapping.remove(&ips.ipv6);
        self.tun_listener_connected_client_tx
            .send(ConnectedClientEvent::Disconnect(DisconnectEvent(*ips)))
            .inspect_err(|err| {
                log::error!("Failed to send disconnect event: {err}");
            })
            .ok();
    }

    fn disconnect_stopped_client_handlers(
        &mut self,
        stopped_clients: Vec<(IpPair, Recipient, SupportedClientVersion)>,
    ) {
        for (ips, _, _) in &stopped_clients {
            log::info!("Disconnect stopped client: {ips}");
            self.disconnect(ips);
        }
    }

    fn disconnect_inactive_clients(
        &mut self,
        inactive_clients: Vec<(IpPair, Recipient, SupportedClientVersion)>,
    ) {
        for (ips, _, _) in &inactive_clients {
            log::info!("Disconnect inactive client: {ips}");
            self.disconnect(ips);
        }
    }

    fn find_new_ip(&self) -> Option<IpPair> {
        generate_new_ip::find_new_ips(&self.clients_ipv4_mapping, &self.clients_ipv6_mapping)
    }
}

pub(crate) struct CloseTx {
    pub(crate) nym_address: Recipient,
    // Send to connected clients listener to stop. This is option only because we need to take
    // ownership of it when the client is dropped.
    pub(crate) inner: Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Clone)]
pub(crate) struct ConnectedClient {
    // The nym address of the connected client that we are communicating with on the other side of
    // the mixnet
    pub(crate) nym_address: Recipient,

    // The assigned IPv6 address of this client
    pub(crate) ipv6: Ipv6Addr,

    // Keep track of last activity so we can disconnect inactive clients
    pub(crate) last_activity: Arc<RwLock<std::time::Instant>>,

    // The version of the client, since we need to know this to send the correct response
    pub(crate) client_version: SupportedClientVersion,

    pub(crate) _close_tx: Arc<CloseTx>,

    // Handle for the connected client handler
    pub(crate) handle: Arc<tokio::task::JoinHandle<()>>,
}

impl ConnectedClient {
    async fn update_activity(&self) {
        *self.last_activity.write().await = std::time::Instant::now();
    }
}

impl Drop for CloseTx {
    fn drop(&mut self) {
        log::debug!("signal to close client: {}", self.nym_address);
        if let Some(close_tx) = self.inner.take() {
            close_tx.send(()).ok();
        }
    }
}

type PacketHandleResult = Result<Option<Response>>;

#[derive(Debug, Clone)]
enum Response {
    V6(v6::response::IpPacketResponse),
    V7(v7::response::IpPacketResponse),
}

impl Response {
    fn recipient(&self) -> Option<&Recipient> {
        match self {
            Response::V6(response) => response.recipient(),
            Response::V7(response) => response.recipient(),
        }
    }

    fn new_static_connect_success(
        request_id: u64,
        reply_to: Recipient,
        client_version: SupportedClientVersion,
    ) -> Self {
        match client_version {
            SupportedClientVersion::V6 => Response::V6(
                v6::response::IpPacketResponse::new_static_connect_success(request_id, reply_to),
            ),
            SupportedClientVersion::V7 => Response::V7(
                v7::response::IpPacketResponse::new_static_connect_success(request_id, reply_to),
            ),
        }
    }

    fn new_static_connect_failure(
        request_id: u64,
        reply_to: Recipient,
        reason: StaticConnectFailureReason,
        client_version: SupportedClientVersion,
    ) -> Self {
        match client_version {
            SupportedClientVersion::V6 => {
                Response::V6(v6::response::IpPacketResponse::new_static_connect_failure(
                    request_id,
                    reply_to,
                    reason.into(),
                ))
            }
            SupportedClientVersion::V7 => {
                Response::V7(v7::response::IpPacketResponse::new_static_connect_failure(
                    request_id, reply_to, reason,
                ))
            }
        }
    }

    fn new_dynamic_connect_success(
        request_id: u64,
        reply_to: Recipient,
        ips: IpPair,
        client_version: SupportedClientVersion,
    ) -> Self {
        match client_version {
            SupportedClientVersion::V6 => {
                Response::V6(v6::response::IpPacketResponse::new_dynamic_connect_success(
                    request_id, reply_to, ips,
                ))
            }
            SupportedClientVersion::V7 => {
                Response::V7(v7::response::IpPacketResponse::new_dynamic_connect_success(
                    request_id, reply_to, ips,
                ))
            }
        }
    }

    fn new_dynamic_connect_failure(
        request_id: u64,
        reply_to: Recipient,
        reason: DynamicConnectFailureReason,
        client_version: SupportedClientVersion,
    ) -> Self {
        match client_version {
            SupportedClientVersion::V6 => {
                Response::V6(v6::response::IpPacketResponse::new_dynamic_connect_failure(
                    request_id,
                    reply_to,
                    reason.into(),
                ))
            }
            SupportedClientVersion::V7 => {
                Response::V7(v7::response::IpPacketResponse::new_dynamic_connect_failure(
                    request_id, reply_to, reason,
                ))
            }
        }
    }

    fn new_data_info_response(
        reply_to: Recipient,
        reply: InfoResponseReply,
        level: InfoLevel,
        client_version: SupportedClientVersion,
    ) -> Self {
        match client_version {
            SupportedClientVersion::V6 => {
                Response::V6(v6::response::IpPacketResponse::new_data_info_response(
                    reply_to,
                    reply.into(),
                    level.into(),
                ))
            }
            SupportedClientVersion::V7 => Response::V7(
                v7::response::IpPacketResponse::new_data_info_response(reply_to, reply, level),
            ),
        }
    }

    fn new_pong(
        request_id: u64,
        reply_to: Recipient,
        client_version: SupportedClientVersion,
    ) -> Self {
        match client_version {
            SupportedClientVersion::V6 => Response::V6(v6::response::IpPacketResponse::new_pong(
                request_id, reply_to,
            )),
            SupportedClientVersion::V7 => Response::V7(v7::response::IpPacketResponse::new_pong(
                request_id, reply_to,
            )),
        }
    }

    fn new_health_response(
        request_id: u64,
        reply_to: Recipient,
        build_info: nym_bin_common::build_information::BinaryBuildInformationOwned,
        client_version: SupportedClientVersion,
    ) -> Self {
        match client_version {
            SupportedClientVersion::V6 => {
                Response::V6(v6::response::IpPacketResponse::new_health_response(
                    request_id, reply_to, build_info, None,
                ))
            }
            SupportedClientVersion::V7 => {
                Response::V7(v7::response::IpPacketResponse::new_health_response(
                    request_id, reply_to, build_info, None,
                ))
            }
        }
    }

    fn new_unrequested_disconnect(
        reply_to: Recipient,
        reason: v7::response::UnrequestedDisconnectReason,
        client_version: SupportedClientVersion,
    ) -> Self {
        match client_version {
            SupportedClientVersion::V6 => Response::V6(
                v6::response::IpPacketResponse::new_unrequested_disconnect(reply_to, reason.into()),
            ),
            SupportedClientVersion::V7 => Response::V7(
                v7::response::IpPacketResponse::new_unrequested_disconnect(reply_to, reason),
            ),
        }
    }

    fn to_bytes(&self) -> Result<Vec<u8>> {
        match self {
            Response::V6(response) => response.to_bytes(),
            Response::V7(response) => response.to_bytes(),
        }
        .map_err(|err| {
            log::error!("Failed to serialize response packet");
            IpPacketRouterError::FailedToSerializeResponsePacket { source: err }
        })
    }
}

#[cfg(target_os = "linux")]
pub(crate) struct MixnetListener {
    // The configuration for the mixnet listener
    pub(crate) _config: Config,

    // The request filter that we use to check if a packet should be forwarded
    pub(crate) request_filter: request_filter::RequestFilter,

    // The TUN device that we use to send and receive packets from the internet
    pub(crate) tun_writer: tokio::io::WriteHalf<tokio_tun::Tun>,

    // The mixnet client that we use to send and receive packets from the mixnet
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,

    // The task handle for the main loop
    pub(crate) task_handle: TaskHandle,

    // The map of connected clients that the mixnet listener keeps track of. It monitors
    // activity and disconnects clients that have been inactive for too long.
    pub(crate) connected_clients: ConnectedClients,
}

#[cfg(target_os = "linux")]
impl MixnetListener {
    // Receving a static connect request from a client with an IP provided that we assign to them,
    // if it's available. If it's not available, we send a failure response.
    async fn on_static_connect_request(
        &mut self,
        connect_request: StaticConnectRequest,
        client_version: SupportedClientVersion,
    ) -> PacketHandleResult {
        log::info!(
            "Received static connect request from {sender_address}",
            sender_address = connect_request.reply_to
        );

        let request_id = connect_request.request_id;
        let requested_ips = connect_request.ips;
        let reply_to = connect_request.reply_to;
        // TODO: add to connect request
        let buffer_timeout = nym_ip_packet_requests::codec::BUFFER_TIMEOUT;

        // Check that the IP is available in the set of connected clients
        let is_ip_taken = self.connected_clients.is_ip_connected(&requested_ips);

        // Check that the nym address isn't already registered
        let is_nym_address_taken = self.connected_clients.is_nym_address_connected(&reply_to);

        match (is_ip_taken, is_nym_address_taken) {
            (true, true) => {
                log::info!("Connecting an already connected client");
                if self
                    .connected_clients
                    .update_activity(&requested_ips)
                    .await
                    .is_err()
                {
                    log::error!("Failed to update activity for client");
                };
                Ok(Some(Response::new_static_connect_success(
                    request_id,
                    reply_to,
                    client_version,
                )))
            }
            (false, false) => {
                log::info!("Connecting a new client");

                // Spawn the ConnectedClientHandler for the new client
                let (forward_from_tun_tx, close_tx, handle) =
                    connected_client_handler::ConnectedClientHandler::start(
                        reply_to,
                        buffer_timeout,
                        client_version,
                        self.mixnet_client.split_sender(),
                    );

                // Register the new client in the set of connected clients
                self.connected_clients.connect(
                    requested_ips,
                    reply_to,
                    client_version,
                    forward_from_tun_tx,
                    close_tx,
                    handle,
                );
                Ok(Some(Response::new_static_connect_success(
                    request_id,
                    reply_to,
                    client_version,
                )))
            }
            (true, false) => {
                log::info!("Requested IP is not available");
                Ok(Some(Response::new_static_connect_failure(
                    request_id,
                    reply_to,
                    StaticConnectFailureReason::RequestedIpAlreadyInUse,
                    client_version,
                )))
            }
            (false, true) => {
                log::info!("Nym address is already registered");
                Ok(Some(Response::new_static_connect_failure(
                    request_id,
                    reply_to,
                    StaticConnectFailureReason::RequestedNymAddressAlreadyInUse,
                    client_version,
                )))
            }
        }
    }

    async fn on_dynamic_connect_request(
        &mut self,
        connect_request: DynamicConnectRequest,
        client_version: SupportedClientVersion,
    ) -> PacketHandleResult {
        log::info!(
            "Received dynamic connect request from {sender_address}",
            sender_address = connect_request.reply_to
        );

        let request_id = connect_request.request_id;
        let reply_to = connect_request.reply_to;
        // TODO: add to connect request
        let buffer_timeout = nym_ip_packet_requests::codec::BUFFER_TIMEOUT;

        // Check if it's the same client connecting again, then we just reuse the same IP
        // TODO: this is problematic. Until we sign connect requests this means you can spam people
        // with return traffic

        if let Some(existing_ips) = self.connected_clients.lookup_ip_from_nym_address(&reply_to) {
            log::info!("Found existing client for nym address");
            if self
                .connected_clients
                .update_activity(&existing_ips)
                .await
                .is_err()
            {
                log::error!("Failed to update activity for client");
            }
            return Ok(Some(Response::new_dynamic_connect_success(
                request_id,
                reply_to,
                existing_ips,
                client_version,
            )));
        }

        let Some(new_ips) = self.connected_clients.find_new_ip() else {
            log::info!("No available IP address");
            return Ok(Some(Response::new_dynamic_connect_failure(
                request_id,
                reply_to,
                DynamicConnectFailureReason::NoAvailableIp,
                client_version,
            )));
        };

        // Spawn the ConnectedClientHandler for the new client
        let (forward_from_tun_tx, close_tx, handle) =
            connected_client_handler::ConnectedClientHandler::start(
                reply_to,
                buffer_timeout,
                client_version,
                self.mixnet_client.split_sender(),
            );

        // Register the new client in the set of connected clients
        self.connected_clients.connect(
            new_ips,
            reply_to,
            client_version,
            forward_from_tun_tx,
            close_tx,
            handle,
        );
        Ok(Some(Response::new_dynamic_connect_success(
            request_id,
            reply_to,
            new_ips,
            client_version,
        )))
    }

    fn on_disconnect_request(
        &self,
        disconnect_request: DisconnectRequest,
        client_version: SupportedClientVersion,
    ) -> PacketHandleResult {
        log::info!("Received disconnect request");

        let request_id = disconnect_request.request_id;
        let reply_to = disconnect_request.reply_to;

        let ips = self.connected_clients.lookup_ip_from_nym_address(&reply_to);
        self.connected_clients.disconnect(&ips);

    }

    async fn handle_packet(
        &mut self,
        ip_packet: &Bytes,
        client_version: SupportedClientVersion,
    ) -> PacketHandleResult {
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
        } = parse_packet(ip_packet)?;

        let dst_str = dst.map_or(dst_addr.to_string(), |dst| dst.to_string());
        log::debug!("Received packet: {packet_type}: {src_addr} -> {dst_str}");

        if let Some(connected_client) = self.connected_clients.get_client_from_ip_mut(&src_addr) {
            // Keep track of activity so we can disconnect inactive clients
            connected_client.update_activity().await;

            // For packets without a port, use 0.
            let dst = dst.unwrap_or_else(|| SocketAddr::new(dst_addr, 0));

            // Filter check
            if self.request_filter.check_address(&dst).await {
                // Forward the packet to the TUN device where it will be routed out to the internet
                self.tun_writer
                    .write_all(ip_packet)
                    .await
                    .map_err(|_| IpPacketRouterError::FailedToWritePacketToTun)?;
                Ok(None)
            } else {
                log::info!("Denied filter check: {dst}");
                Ok(Some(Response::new_data_info_response(
                    connected_client.nym_address,
                    InfoResponseReply::ExitPolicyFilterCheckFailed {
                        dst: dst.to_string(),
                    },
                    InfoLevel::Warn,
                    client_version,
                )))
            }
        } else {
            // If the client is not connected, just drop the packet silently
            log::info!("dropping packet from mixnet: no registered client for packet with source: {src_addr}");
            Ok(None)
        }
    }

    async fn on_data_request(
        &mut self,
        data_request: DataRequest,
        client_version: SupportedClientVersion,
    ) -> Result<Vec<PacketHandleResult>> {
        let mut responses = Vec::new();
        let mut decoder = MultiIpPacketCodec::new(nym_ip_packet_requests::codec::BUFFER_TIMEOUT);
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&data_request.ip_packets);
        while let Ok(Some(packet)) = decoder.decode(&mut bytes) {
            let result = self.handle_packet(&packet, client_version).await;
            responses.push(result);
        }
        Ok(responses)
    }

    fn on_ping_request(
        &self,
        ping_request: PingRequest,
        client_version: SupportedClientVersion,
    ) -> PacketHandleResult {
        log::info!(
            "Received ping request from {sender_address}",
            sender_address = ping_request.reply_to
        );

        let reply_to = ping_request.reply_to;
        let request_id = ping_request.request_id;

        Ok(Some(Response::new_pong(
            request_id,
            reply_to,
            client_version,
        )))
    }

    fn on_health_request(
        &self,
        health_request: HealthRequest,
        client_version: SupportedClientVersion,
    ) -> PacketHandleResult {
        log::info!(
            "Received health request from {sender_address}",
            sender_address = health_request.reply_to
        );

        let reply_to = health_request.reply_to;
        let request_id = health_request.request_id;
        let build_info = nym_bin_common::bin_info_owned!();

        Ok(Some(Response::new_health_response(
            request_id,
            reply_to,
            build_info,
            client_version,
        )))
    }

    fn on_version_mismatch(
        &self,
        _version: u8,
        _reconstructed: &ReconstructedMessage,
    ) -> PacketHandleResult {
        // Just drop it. In the future we might want to return a response here, if for example
        // the client is connecting with a version that is older than the currently supported
        // ones.
        Ok(None)
    }

    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> Result<Vec<PacketHandleResult>> {
        log::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );

        let (request, client_version) = match deserialize_request(&reconstructed) {
            Err(IpPacketRouterError::InvalidPacketVersion(version)) => {
                return Ok(vec![self.on_version_mismatch(version, &reconstructed)]);
            }
            req => req,
        }?;

        match request.data {
            IpPacketRequestData::StaticConnect(signed_connect_request) => {
                verify_signed_request(&signed_connect_request, client_version)?;
                let connect_request = signed_connect_request.request;
                Ok(vec![
                    self.on_static_connect_request(connect_request, client_version)
                        .await,
                ])
            }
            IpPacketRequestData::DynamicConnect(signed_connect_request) => {
                verify_signed_request(&signed_connect_request, client_version)?;
                let connect_request = signed_connect_request.request;
                Ok(vec![
                    self.on_dynamic_connect_request(connect_request, client_version)
                        .await,
                ])
            }
            IpPacketRequestData::Disconnect(signed_disconnect_request) => {
                verify_signed_request(&signed_disconnect_request, client_version)?;
                let disconnect_request = signed_disconnect_request.request;
                Ok(vec![
                    self.on_disconnect_request(disconnect_request, client_version)
                ])
            }
            IpPacketRequestData::Data(data_request) => {
                self.on_data_request(data_request, client_version).await
            }
            IpPacketRequestData::Ping(ping_request) => {
                Ok(vec![self.on_ping_request(ping_request, client_version)])
            }
            IpPacketRequestData::Health(health_request) => {
                Ok(vec![self.on_health_request(health_request, client_version)])
            }
        }
    }

    async fn handle_disconnect_timer(&mut self) {
        let stopped_clients = self.connected_clients.get_finished_client_handlers();
        let inactive_clients = self.connected_clients.get_inactive_clients().await;

        // WIP(JON): confirm we should send disconnect on handle stopped
        for (_ip, nym_address, client_version) in &stopped_clients {
            let response = Response::new_unrequested_disconnect(
                *nym_address,
                v7::response::UnrequestedDisconnectReason::Other("handler stopped".to_string()),
                *client_version,
            );
            if let Err(err) = self.handle_response(response).await {
                log::error!("Failed to send disconnect response: {err}");
            }
        }
        for (_ip, nym_address, client_version) in &inactive_clients {
            let response = Response::new_unrequested_disconnect(
                *nym_address,
                v7::response::UnrequestedDisconnectReason::ClientMixnetTrafficTimeout,
                *client_version,
            );
            if let Err(err) = self.handle_response(response).await {
                log::error!("Failed to send disconnect response: {err}");
            }
        }

        self.connected_clients
            .disconnect_stopped_client_handlers(stopped_clients);
        self.connected_clients
            .disconnect_inactive_clients(inactive_clients);
    }

    // When an incoming mixnet message triggers a response that we send back, such as during
    // connect handshake.
    async fn handle_response(&self, response: Response) -> Result<()> {
        let Some(recipient) = response.recipient() else {
            log::error!("No recipient in response packet, this should NOT happen!");
            return Err(IpPacketRouterError::NoRecipientInResponse);
        };

        let response_packet = response.to_bytes()?;

        let input_message = create_input_message(*recipient, response_packet);
        self.mixnet_client
            .send(input_message)
            .await
            .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })
    }

    // A single incoming request can trigger multiple responses, such as when data requests contain
    // multiple IP packets.
    async fn handle_responses(&self, responses: Vec<PacketHandleResult>) {
        for response in responses {
            match response {
                Ok(Some(response)) => {
                    if let Err(err) = self.handle_response(response).await {
                        log::error!("Mixnet listener failed to handle response: {err}");
                    }
                }
                Ok(None) => {
                    continue;
                }
                Err(err) => {
                    log::error!("Error handling mixnet message: {err}");
                }
            }
        }
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        let mut task_client = self.task_handle.fork("main_loop");
        let mut disconnect_timer = tokio::time::interval(DISCONNECT_TIMER_INTERVAL);

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("IpPacketRouter [main loop]: received shutdown");
                },
                _ = disconnect_timer.tick() => {
                    self.handle_disconnect_timer().await;
                },
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        match self.on_reconstructed_message(msg).await {
                            Ok(responses) => self.handle_responses(responses).await,
                            Err(err) => {
                                log::error!("Error handling reconstructed mixnet message: {err}");
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

fn deserialize_request(
    reconstructed: &ReconstructedMessage,
) -> Result<(IpPacketRequest, SupportedClientVersion)> {
    let request_version = *reconstructed
        .message
        .first()
        .ok_or(IpPacketRouterError::EmptyPacket)?;

    // Check version of the request and convert to the latest version if necessary
    let request = match request_version {
        6 => nym_ip_packet_requests::v6::request::IpPacketRequest::from_reconstructed_message(
            reconstructed,
        )
        .map_err(|err| IpPacketRouterError::FailedToDeserializeTaggedPacket { source: err })
        .map(|r| r.into()),
        7 => nym_ip_packet_requests::v7::request::IpPacketRequest::from_reconstructed_message(
            reconstructed,
        )
        .map_err(|err| IpPacketRouterError::FailedToDeserializeTaggedPacket { source: err }),
        _ => {
            log::info!("Received packet with invalid version: v{request_version}");
            Err(IpPacketRouterError::InvalidPacketVersion(request_version))
        }
    };

    let Some(request_version) = SupportedClientVersion::new(request_version) else {
        return Err(IpPacketRouterError::InvalidPacketVersion(request_version));
    };

    // Tag the request with the version of the request
    request.map(|r| (r, request_version))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SupportedClientVersion {
    V6,
    V7,
}

impl SupportedClientVersion {
    fn new(request_version: u8) -> Option<Self> {
        match request_version {
            6 => Some(SupportedClientVersion::V6),
            7 => Some(SupportedClientVersion::V7),
            _ => None,
        }
    }
}

fn verify_signed_request(
    request: &impl SignedRequest,
    client_version: SupportedClientVersion,
) -> Result<()> {
    if let Err(err) = request.verify() {
        // If the client is V6, we don't care about missing signature
        if client_version == SupportedClientVersion::V6 {
            return Ok(());
        }
        return Err(IpPacketRouterError::FailedToVerifyRequest { source: err });
    }
    Ok(())
}

pub(crate) enum ConnectedClientEvent {
    Disconnect(DisconnectEvent),
    Connect(Box<ConnectEvent>),
}

pub(crate) struct DisconnectEvent(pub(crate) IpPair);

pub(crate) struct ConnectEvent {
    pub(crate) ips: IpPair,
    pub(crate) forward_from_tun_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}
