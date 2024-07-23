use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr};

use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use nym_ip_packet_requests::v7::response::{DynamicConnectFailureReason, InfoLevel, InfoResponseReply, IpPacketResponse, StaticConnectFailureReason};
use nym_ip_packet_requests::{
    codec::MultiIpPacketCodec,
    v7::{
        self,
        request::{
            DataRequest, DisconnectRequest, DynamicConnectRequest, IpPacketRequest,
            IpPacketRequestData, StaticConnectRequest,
        },
        signature::{SignatureError, SignedRequest},
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
        mix_hops: Option<u8>,
        forward_from_tun_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        close_tx: tokio::sync::oneshot::Sender<()>,
        handle: tokio::task::JoinHandle<()>,
    ) {
        // The map of connected clients that the mixnet listener keeps track of. It monitors
        // activity and disconnects clients that have been inactive for too long.
        let client = ConnectedClient {
            nym_address,
            ipv6: ips.ipv6,
            mix_hops,
            last_activity: Arc::new(RwLock::new(std::time::Instant::now())),
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
    fn get_finished_client_handlers(&mut self) -> Vec<(IpPair, Recipient)> {
        self.clients_ipv4_mapping
            .iter_mut()
            .filter_map(|(ip, connected_client)| {
                if connected_client.handle.is_finished() {
                    Some((
                        IpPair::new(*ip, connected_client.ipv6),
                        connected_client.nym_address,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    async fn get_inactive_clients(&mut self) -> Vec<(IpPair, Recipient)> {
        let now = std::time::Instant::now();
        let mut ret = vec![];
        for (ip, connected_client) in self.clients_ipv4_mapping.iter() {
            if now.duration_since(*connected_client.last_activity.read().await)
                > CLIENT_MIXNET_INACTIVITY_TIMEOUT
            {
                ret.push((
                    IpPair::new(*ip, connected_client.ipv6),
                    connected_client.nym_address,
                ))
            }
        }
        ret
    }

    fn disconnect_stopped_client_handlers(&mut self, stopped_clients: Vec<(IpPair, Recipient)>) {
        for (ips, _) in &stopped_clients {
            log::info!("Disconnect stopped client: {ips}");
            self.clients_ipv4_mapping.remove(&ips.ipv4);
            self.clients_ipv6_mapping.remove(&ips.ipv6);
            self.tun_listener_connected_client_tx
                .send(ConnectedClientEvent::Disconnect(DisconnectEvent(*ips)))
                .tap_err(|err| {
                    log::error!("Failed to send disconnect event: {err}");
                })
                .ok();
        }
    }

    fn disconnect_inactive_clients(&mut self, inactive_clients: Vec<(IpPair, Recipient)>) {
        for (ips, _) in &inactive_clients {
            log::info!("Disconnect inactive client: {ips}");
            self.clients_ipv4_mapping.remove(&ips.ipv4);
            self.clients_ipv6_mapping.remove(&ips.ipv6);
            self.tun_listener_connected_client_tx
                .send(ConnectedClientEvent::Disconnect(DisconnectEvent(*ips)))
                .tap_err(|err| {
                    log::error!("Failed to send disconnect event: {err}");
                })
                .ok();
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

    // Number of mix node hops that the client has requested to use
    pub(crate) mix_hops: Option<u8>,

    // Keep track of last activity so we can disconnect inactive clients
    pub(crate) last_activity: Arc<RwLock<std::time::Instant>>,

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

type PacketHandleResult = Result<Option<v7::response::IpPacketResponse>>;

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
        client_version: u8,
    ) -> PacketHandleResult {
        log::info!(
            "Received static connect request from {sender_address}",
            sender_address = connect_request.reply_to
        );

        let request_id = connect_request.request_id;
        let requested_ips = connect_request.ips;
        let reply_to = connect_request.reply_to;
        let reply_to_hops = connect_request.reply_to_hops;
        // TODO: add to connect request
        let buffer_timeout = nym_ip_packet_requests::codec::BUFFER_TIMEOUT;
        // TODO: ignoring reply_to_avg_mix_delays for now

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
                Ok(Some(IpPacketResponse::new_static_connect_success(
                    request_id, reply_to,
                )))
            }
            (false, false) => {
                log::info!("Connecting a new client");

                // Spawn the ConnectedClientHandler for the new client
                let (forward_from_tun_tx, close_tx, handle) =
                    connected_client_handler::ConnectedClientHandler::start(
                        reply_to,
                        reply_to_hops,
                        buffer_timeout,
                        client_version,
                        self.mixnet_client.split_sender(),
                    );

                // Register the new client in the set of connected clients
                self.connected_clients.connect(
                    requested_ips,
                    reply_to,
                    reply_to_hops,
                    forward_from_tun_tx,
                    close_tx,
                    handle,
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
        connect_request: DynamicConnectRequest,
        client_version: u8,
    ) -> PacketHandleResult {
        log::info!(
            "Received dynamic connect request from {sender_address}",
            sender_address = connect_request.reply_to
        );

        let request_id = connect_request.request_id;
        let reply_to = connect_request.reply_to;
        let reply_to_hops = connect_request.reply_to_hops;
        // TODO: add to connect request
        let buffer_timeout = nym_ip_packet_requests::codec::BUFFER_TIMEOUT;
        // TODO: ignoring reply_to_avg_mix_delays for now

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
            return Ok(Some(IpPacketResponse::new_dynamic_connect_success(
                request_id,
                reply_to,
                existing_ips,
            )));
        }

        let Some(new_ips) = self.connected_clients.find_new_ip() else {
            log::info!("No available IP address");
            return Ok(Some(IpPacketResponse::new_dynamic_connect_failure(
                request_id,
                reply_to,
                DynamicConnectFailureReason::NoAvailableIp,
            )));
        };

        // Spawn the ConnectedClientHandler for the new client
        let (forward_from_tun_tx, close_tx, handle) =
            connected_client_handler::ConnectedClientHandler::start(
                reply_to,
                reply_to_hops,
                buffer_timeout,
                client_version,
                self.mixnet_client.split_sender(),
            );

        // Register the new client in the set of connected clients
        self.connected_clients.connect(
            new_ips,
            reply_to,
            reply_to_hops,
            forward_from_tun_tx,
            close_tx,
            handle,
        );
        Ok(Some(IpPacketResponse::new_dynamic_connect_success(
            request_id, reply_to, new_ips,
        )))
    }

    fn on_disconnect_request(
        &self,
        _disconnect_request: DisconnectRequest,
        _client_version: u8,
    ) -> PacketHandleResult {
        log::info!("Received disconnect request: not implemented, dropping");
        Ok(None)
    }

    async fn handle_packet(&mut self, ip_packet: &Bytes) -> PacketHandleResult {
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
                Ok(Some(IpPacketResponse::new_data_info_response(
                    connected_client.nym_address,
                    InfoResponseReply::ExitPolicyFilterCheckFailed {
                        dst: dst.to_string(),
                    },
                    InfoLevel::Warn,
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
    ) -> Result<Vec<PacketHandleResult>> {
        let mut responses = Vec::new();
        let mut decoder = MultiIpPacketCodec::new(nym_ip_packet_requests::codec::BUFFER_TIMEOUT);
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&data_request.ip_packets);
        while let Ok(Some(packet)) = decoder.decode(&mut bytes) {
            let result = self.handle_packet(&packet).await;
            responses.push(result);
        }
        Ok(responses)
    }

    fn on_version_mismatch(
        &self,
        version: u8,
        reconstructed: &ReconstructedMessage,
    ) -> PacketHandleResult {
        // If it's possible to parse, do so and return back a response, otherwise just drop
        let (id, recipient) =
            nym_ip_packet_requests::v6::request::IpPacketRequest::from_reconstructed_message(reconstructed)
                .ok()
                .and_then(|request| {
                    request
                        .recipient()
                        .map(|recipient| (request.id().unwrap_or(0), *recipient))
                })
                .ok_or(IpPacketRouterError::InvalidPacketVersion(version))?;

        Ok(Some(IpPacketResponse::new_version_mismatch(
            id,
            recipient,
            version,
            nym_ip_packet_requests::CURRENT_VERSION,
        )))
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
                verify_signed_request(&signed_connect_request)?;
                let connect_request = signed_connect_request.request;
                Ok(vec![
                    self.on_static_connect_request(connect_request, client_version)
                        .await,
                ])
            }
            IpPacketRequestData::DynamicConnect(signed_connect_request) => {
                verify_signed_request(&signed_connect_request)?;
                let connect_request = signed_connect_request.request;
                Ok(vec![
                    self.on_dynamic_connect_request(connect_request, client_version)
                        .await,
                ])
            }
            IpPacketRequestData::Disconnect(signed_disconnect_request) => {
                verify_signed_request(&signed_disconnect_request)?;
                let disconnect_request = signed_disconnect_request.request;
                Ok(vec![
                    self.on_disconnect_request(disconnect_request, client_version)
                ])
            }
            IpPacketRequestData::Data(data_request) => self.on_data_request(data_request).await,
            IpPacketRequestData::Ping(_) => {
                log::info!("Received ping request: not implemented, dropping");
                Ok(vec![])
            }
            IpPacketRequestData::Health(_) => {
                log::info!("Received health request: not implemented, dropping");
                Ok(vec![])
            }
        }
    }

    async fn handle_disconnect_timer(&mut self) {
        let stopped_clients = self.connected_clients.get_finished_client_handlers();
        let inactive_clients = self.connected_clients.get_inactive_clients().await;

        // TODO: Send disconnect responses to all disconnected clients
        //for (ip, nym_address) in stopped_clients.iter().chain(disconnected_clients.iter()) {
        //    let response = IpPacketResponse::new_unrequested_disconnect(...)
        //    if let Err(err) = self.handle_response(response).await {
        //        log::error!("Failed to send disconnect response: {err}");
        //    }
        //}

        self.connected_clients
            .disconnect_stopped_client_handlers(stopped_clients);
        self.connected_clients
            .disconnect_inactive_clients(inactive_clients);
    }

    // When an incoming mixnet message triggers a response that we send back, such as during
    // connect handshake.
    async fn handle_response(&self, response: IpPacketResponse) -> Result<()> {
        // Convert to earlier version if needed
        let response: nym_ip_packet_requests::v6::response::IpPacketResponse = response.into();

        let Some(recipient) = response.recipient() else {
            log::error!("No recipient in response packet, this should NOT happen!");
            return Err(IpPacketRouterError::NoRecipientInResponse);
        };

        let response_packet = response.to_bytes().map_err(|err| {
            log::error!("Failed to serialize response packet");
            IpPacketRouterError::FailedToSerializeResponsePacket { source: err }
        })?;

        // We could avoid this lookup if we check this when we create the response.
        let mix_hops = if let Some(c) = self
            .connected_clients
            .lookup_client_from_nym_address(recipient)
        {
            c.mix_hops
        } else {
            None
        };

        let input_message = create_input_message(*recipient, response_packet, mix_hops);
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

fn deserialize_request(reconstructed: &ReconstructedMessage) -> Result<(IpPacketRequest, u8)> {
    let request_version = *reconstructed
        .message
        .first()
        .ok_or(IpPacketRouterError::EmptyPacket)?;

    // Check version of the request and convert to the latest version if necessary
    let request = match request_version {
        6 => nym_ip_packet_requests::v6::request::IpPacketRequest::from_reconstructed_message(reconstructed)
            .map_err(|err| IpPacketRouterError::FailedToDeserializeTaggedPacket { source: err })
            .map(|r| r.into()),
        7 => nym_ip_packet_requests::v7::request::IpPacketRequest::from_reconstructed_message(reconstructed)
            .map_err(|err| IpPacketRouterError::FailedToDeserializeTaggedPacket { source: err }),
        _ => {
            log::info!("Received packet with invalid version: v{request_version}");
            Err(IpPacketRouterError::InvalidPacketVersion(request_version))
        }
    };

    // Tag the request with the version of the request
    request.map(|r| (r, request_version))
}

fn verify_signed_request(request: &impl SignedRequest) -> Result<()> {
    if let Err(err) = request.verify() {
        // Once we start to require clients to send v7 requests, we will enfore checking
        // signatures. Until then, we only check if they are present.
        if !matches!(err, SignatureError::MissingSignature) {
            return Err(IpPacketRouterError::FailedToVerifyRequest { source: err });
        }
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
