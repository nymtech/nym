use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
};

use futures::StreamExt;
use nym_ip_packet_requests::{
    request::{IpPacketRequest, IpPacketRequestData},
    response::{
        DynamicConnectFailureReason, ErrorResponseReply, IpPacketResponse,
        StaticConnectFailureReason,
    },
};
use nym_sdk::mixnet::{MixnetMessageSender, Recipient};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;
use tap::TapFallible;
#[cfg(target_os = "linux")]
use tokio::io::AsyncWriteExt;

use crate::{
    config::Config,
    connected_client_handler,
    constants::{CLIENT_INACTIVITY_TIMEOUT, DISCONNECT_TIMER_INTERVAL},
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
    clients: HashMap<IpAddr, ConnectedClient>,

    // Notify the tun listener when a new client connects or disconnects
    tun_listener_connected_client_tx: tokio::sync::mpsc::UnboundedSender<ConnectedClientEvent>,
}

impl ConnectedClients {
    pub(crate) fn new() -> (Self, tun_listener::ConnectedClientsListener) {
        let (connected_client_tx, connected_client_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                clients: Default::default(),
                tun_listener_connected_client_tx: connected_client_tx,
            },
            tun_listener::ConnectedClientsListener::new(connected_client_rx),
        )
    }

    fn is_ip_connected(&self, ip: &IpAddr) -> bool {
        self.clients.contains_key(ip)
    }

    fn get_client_from_ip_mut(&mut self, ip: &IpAddr) -> Option<&mut ConnectedClient> {
        self.clients.get_mut(ip)
    }

    fn is_nym_address_connected(&self, nym_address: &Recipient) -> bool {
        self.clients
            .values()
            .any(|client| client.nym_address == *nym_address)
    }

    fn lookup_ip_from_nym_address(&self, nym_address: &Recipient) -> Option<IpAddr> {
        self.clients.iter().find_map(|(ip, client)| {
            if client.nym_address == *nym_address {
                Some(*ip)
            } else {
                None
            }
        })
    }

    fn lookup_client_from_nym_address(&self, nym_address: &Recipient) -> Option<&ConnectedClient> {
        self.clients
            .values()
            .find(|client| client.nym_address == *nym_address)
    }

    fn connect(
        &mut self,
        ip: IpAddr,
        nym_address: Recipient,
        mix_hops: Option<u8>,
        forward_from_tun_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        close_tx: tokio::sync::oneshot::Sender<()>,
        finished_rx: tokio::sync::oneshot::Receiver<()>,
    ) {
        // The map of connected clients that the mixnet listener keeps track of. It monitors
        // activity and disconnects clients that have been inactive for too long.
        self.clients.insert(
            ip,
            ConnectedClient {
                nym_address,
                mix_hops,
                last_activity: std::time::Instant::now(),
                close_tx: Some(close_tx),
                finished_rx,
            },
        );
        // Send the connected client info to the tun listener, which will use it to forward packets
        // to the connected client handler.
        self.tun_listener_connected_client_tx
            .send(ConnectedClientEvent::Connect(Box::new(ConnectEvent {
                ip,
                forward_from_tun_tx,
            })))
            .tap_err(|err| {
                log::error!("Failed to send connected client event: {err}");
            })
            .ok();
    }

    fn update_activity(&mut self, ip: &IpAddr) -> Result<()> {
        if let Some(client) = self.clients.get_mut(ip) {
            client.last_activity = std::time::Instant::now();
            Ok(())
        } else {
            Err(IpPacketRouterError::FailedToUpdateClientActivity)
        }
    }

    fn get_stopped_client_handlers(&mut self) -> Vec<(IpAddr, Recipient)> {
        self.clients
            .iter_mut()
            .filter_map(|(ip, client)| {
                if client.finished_rx.try_recv().is_ok() {
                    Some((*ip, client.nym_address))
                } else {
                    None
                }
            })
            .collect()
    }

    fn get_inactive_clients(&mut self) -> Vec<(IpAddr, Recipient)> {
        let now = std::time::Instant::now();
        self.clients
            .iter()
            .filter_map(|(ip, client)| {
                if now.duration_since(client.last_activity) > CLIENT_INACTIVITY_TIMEOUT {
                    Some((*ip, client.nym_address))
                } else {
                    None
                }
            })
            .collect()
    }

    fn disconnect_stopped_client_handlers(&mut self, stopped_clients: Vec<(IpAddr, Recipient)>) {
        for (ip, _) in &stopped_clients {
            log::info!("Removing stopped client: {ip}");
            self.clients.remove(ip);
            self.tun_listener_connected_client_tx
                .send(ConnectedClientEvent::Disconnect(DisconnectEvent(*ip)))
                .tap_err(|err| {
                    log::error!("Failed to send disconnect event: {err}");
                })
                .ok();
        }
    }

    fn disconnect_inactive_clients(&mut self, inactive_clients: Vec<(IpAddr, Recipient)>) {
        for (ip, _) in &inactive_clients {
            log::info!("Disconnect inactive client: {ip}");
            self.clients.remove(ip);
            self.tun_listener_connected_client_tx
                .send(ConnectedClientEvent::Disconnect(DisconnectEvent(*ip)))
                .tap_err(|err| {
                    log::error!("Failed to send disconnect event: {err}");
                })
                .ok();
        }
    }

    fn find_new_ip(&self) -> Option<IpAddr> {
        generate_new_ip::find_new_ip(&self.clients)
    }
}

pub(crate) struct ConnectedClient {
    // The nym address of the connected client that we are communicating with on the other side of
    // the mixnet
    pub(crate) nym_address: Recipient,

    // Number of mix node hops that the client has requested to use
    pub(crate) mix_hops: Option<u8>,

    // Keep track of last activity so we can disconnect inactive clients
    pub(crate) last_activity: std::time::Instant,

    // Send to connected clients listener to stop. This is option only because we need to take
    // ownership of it when the client is dropped.
    pub(crate) close_tx: Option<tokio::sync::oneshot::Sender<()>>,

    // Receive event when the client listener for that client stopped
    pub(crate) finished_rx: tokio::sync::oneshot::Receiver<()>,
}

impl ConnectedClient {
    fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }
}

impl Drop for ConnectedClient {
    fn drop(&mut self) {
        log::info!("Dropping client: {}", self.nym_address);
        if let Some(close_tx) = self.close_tx.take() {
            log::trace!("Sending close signal to connected client handler");
            close_tx.send(()).unwrap();
        }
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
        connect_request: nym_ip_packet_requests::request::StaticConnectRequest,
    ) -> Result<Option<IpPacketResponse>> {
        log::info!(
            "Received static connect request from {sender_address}",
            sender_address = connect_request.reply_to
        );

        let request_id = connect_request.request_id;
        let requested_ip = connect_request.ip;
        let reply_to = connect_request.reply_to;
        let reply_to_hops = connect_request.reply_to_hops;
        // TODO: ignoring reply_to_avg_mix_delays for now

        // Check that the IP is available in the set of connected clients
        let is_ip_taken = self.connected_clients.is_ip_connected(&requested_ip);

        // Check that the nym address isn't already registered
        let is_nym_address_taken = self.connected_clients.is_nym_address_connected(&reply_to);

        match (is_ip_taken, is_nym_address_taken) {
            (true, true) => {
                log::info!("Connecting an already connected client");
                if self
                    .connected_clients
                    .update_activity(&requested_ip)
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
                let (forward_from_tun_tx, close_tx, finished_rx) =
                    connected_client_handler::ConnectedClientHandler::start(
                        reply_to,
                        reply_to_hops,
                        self.mixnet_client.split_sender(),
                    );

                // Register the new client in the set of connected clients
                self.connected_clients.connect(
                    requested_ip,
                    reply_to,
                    reply_to_hops,
                    forward_from_tun_tx,
                    close_tx,
                    finished_rx,
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
        connect_request: nym_ip_packet_requests::request::DynamicConnectRequest,
    ) -> Result<Option<IpPacketResponse>> {
        log::info!(
            "Received dynamic connect request from {sender_address}",
            sender_address = connect_request.reply_to
        );

        let request_id = connect_request.request_id;
        let reply_to = connect_request.reply_to;
        let reply_to_hops = connect_request.reply_to_hops;
        // TODO: ignoring reply_to_avg_mix_delays for now

        // Check if it's the same client connecting again, then we just reuse the same IP
        // TODO: this is problematic. Until we sign connect requests this means you can spam people
        // with return traffic

        if let Some(existing_ip) = self.connected_clients.lookup_ip_from_nym_address(&reply_to) {
            log::info!("Found existing client for nym address");
            if self
                .connected_clients
                .update_activity(&existing_ip)
                .is_err()
            {
                log::error!("Failed to update activity for client");
            }
            return Ok(Some(IpPacketResponse::new_dynamic_connect_success(
                request_id,
                reply_to,
                existing_ip,
            )));
        }

        let Some(new_ip) = self.connected_clients.find_new_ip() else {
            log::info!("No available IP address");
            return Ok(Some(IpPacketResponse::new_dynamic_connect_failure(
                request_id,
                reply_to,
                DynamicConnectFailureReason::NoAvailableIp,
            )));
        };

        // Spawn the ConnectedClientHandler for the new client
        let (forward_from_tun_tx, close_tx, finished_rx) =
            connected_client_handler::ConnectedClientHandler::start(
                reply_to,
                reply_to_hops,
                self.mixnet_client.split_sender(),
            );

        // Register the new client in the set of connected clients
        self.connected_clients.connect(
            new_ip,
            reply_to,
            reply_to_hops,
            forward_from_tun_tx,
            close_tx,
            finished_rx,
        );
        Ok(Some(IpPacketResponse::new_dynamic_connect_success(
            request_id, reply_to, new_ip,
        )))
    }

    fn on_disconnect_request(
        &self,
        _disconnect_request: nym_ip_packet_requests::request::DisconnectRequest,
    ) -> Result<Option<IpPacketResponse>> {
        log::info!("Received disconnect request: not implemented, dropping");
        Ok(None)
    }

    async fn on_data_request(
        &mut self,
        data_request: nym_ip_packet_requests::request::DataRequest,
    ) -> Result<Option<IpPacketResponse>> {
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

        if let Some(connected_client) = self.connected_clients.get_client_from_ip_mut(&src_addr) {
            // Keep track of activity so we can disconnect inactive clients
            connected_client.update_activity();

            // For packets without a port, use 0.
            let dst = dst.unwrap_or_else(|| SocketAddr::new(dst_addr, 0));

            // Filter check
            if self.request_filter.check_address(&dst).await {
                // Forward the packet to the TUN device where it will be routed out to the internet
                self.tun_writer
                    .write_all(&data_request.ip_packet)
                    .await
                    .map_err(|_| IpPacketRouterError::FailedToWritePacketToTun)?;
                Ok(None)
            } else {
                log::info!("Denied filter check: {dst}");
                Ok(Some(IpPacketResponse::new_data_error_response(
                    connected_client.nym_address,
                    ErrorResponseReply::ExitPolicyFilterCheckFailed {
                        dst: dst.to_string(),
                    },
                )))
            }
        } else {
            // If the client is not connected, just drop the packet silently
            log::info!("Dropping packet: no connected client for {src_addr}");
            Ok(None)
        }
    }

    fn on_version_mismatch(
        &self,
        version: u8,
        reconstructed: &ReconstructedMessage,
    ) -> Result<Option<IpPacketResponse>> {
        // If it's possible to parse, do so and return back a response, otherwise just drop
        let (id, recipient) = IpPacketRequest::from_reconstructed_message(reconstructed)
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
    ) -> Result<Option<IpPacketResponse>> {
        log::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );

        // Check version of request
        if let Some(version) = reconstructed.message.first() {
            // The idea is that in the future we can add logic here to parse older versions to stay
            // backwards compatible.
            if *version != nym_ip_packet_requests::CURRENT_VERSION {
                log::info!("Received packet with invalid version: v{version}");
                return self.on_version_mismatch(*version, &reconstructed);
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
            IpPacketRequestData::Disconnect(disconnect_request) => {
                self.on_disconnect_request(disconnect_request)
            }
            IpPacketRequestData::Data(data_request) => self.on_data_request(data_request).await,
        }
    }

    fn handle_disconnect_timer(&mut self) {
        let stopped_clients = self.connected_clients.get_stopped_client_handlers();
        let inactive_clients = self.connected_clients.get_inactive_clients();

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
        let Some(recipient) = response.recipient() else {
            log::error!("no recipient in response packet, this should NOT happen!");
            return Err(IpPacketRouterError::NoRecipientInResponse);
        };

        let response_packet = response.to_bytes().map_err(|err| {
            log::error!("Failed to serialize response packet");
            IpPacketRouterError::FailedToSerializeResponsePacket { source: err }
        })?;

        // We could avoid this lookup if we check this when we create the response.
        let mix_hops = self
            .connected_clients
            .lookup_client_from_nym_address(recipient)
            .and_then(|c| c.mix_hops);

        let input_message = create_input_message(*recipient, response_packet, mix_hops);
        self.mixnet_client
            .send(input_message)
            .await
            .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })
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
                    self.handle_disconnect_timer();
                },
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        match self.on_reconstructed_message(msg).await {
                            Ok(Some(response)) => {
                                if let Err(err) = self.handle_response(response).await {
                                    log::error!("Mixnet listener failed to handle response: {err}");
                                }
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
    Disconnect(DisconnectEvent),
    Connect(Box<ConnectEvent>),
}

pub(crate) struct DisconnectEvent(pub(crate) IpAddr);

pub(crate) struct ConnectEvent {
    pub(crate) ip: IpAddr,
    pub(crate) forward_from_tun_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}
