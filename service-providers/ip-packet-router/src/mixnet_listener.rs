use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
};

use futures::StreamExt;
use nym_ip_packet_requests::{
    DynamicConnectFailureReason, IpPacketRequest, IpPacketRequestData, IpPacketResponse,
    StaticConnectFailureReason,
};
use nym_sdk::mixnet::{MixnetMessageSender, Recipient};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;
#[cfg(target_os = "linux")]
use tokio::io::AsyncWriteExt;

use crate::{
    config::Config,
    constants::{CLIENT_INACTIVITY_TIMEOUT, DISCONNECT_TIMER_INTERVAL},
    error::{IpPacketRouterError, Result},
    request_filter::{self},
    util::generate_new_ip,
    util::{
        create_message::create_input_message,
        parse_ip::{parse_packet, ParsedPacket},
    },
};

#[cfg(target_os = "linux")]
pub(crate) struct MixnetListener {
    pub(crate) _config: Config,
    pub(crate) request_filter: request_filter::RequestFilter,
    pub(crate) tun_writer: tokio::io::WriteHalf<tokio_tun::Tun>,
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,
    pub(crate) task_handle: TaskHandle,
    pub(crate) connected_clients: ConnectedClients,
}

pub(crate) struct ConnectedClients {
    clients: HashMap<IpAddr, ConnectedClient>,
    connected_client_tx: tokio::sync::mpsc::UnboundedSender<ConnectedClientEvent>,
}

pub(crate) struct ConnectedClientsListener {
    clients: HashMap<IpAddr, ConnectedClient>,
    pub(crate) connected_client_rx: tokio::sync::mpsc::UnboundedReceiver<ConnectedClientEvent>,
}

impl ConnectedClientsListener {
    pub(crate) fn get(&self, ip: &IpAddr) -> Option<&ConnectedClient> {
        self.clients.get(ip)
    }

    pub(crate) fn update(&mut self, event: ConnectedClientEvent) {
        match event {
            ConnectedClientEvent::Connect(connected_event) => {
                let ConnectEvent {
                    ip,
                    nym_address,
                    mix_hops,
                } = *connected_event;
                log::trace!("Connect client: {ip}");
                self.clients.insert(
                    ip,
                    ConnectedClient {
                        nym_address,
                        mix_hops,
                        last_activity: std::time::Instant::now(),
                    },
                );
            }
            ConnectedClientEvent::Disconnect(DisconnectEvent(ip)) => {
                log::trace!("Disconnect client: {ip}");
                self.clients.remove(&ip);
            }
        }
    }
}

impl ConnectedClients {
    pub(crate) fn new() -> (Self, ConnectedClientsListener) {
        let (connected_client_tx, connected_client_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                clients: Default::default(),
                connected_client_tx,
            },
            ConnectedClientsListener {
                clients: Default::default(),
                connected_client_rx,
            },
        )
    }

    fn is_ip_connected(&self, ip: &IpAddr) -> bool {
        self.clients.contains_key(ip)
    }

    fn is_nym_address_connected(&self, nym_address: &Recipient) -> bool {
        self.clients
            .values()
            .any(|client| client.nym_address == *nym_address)
    }

    fn get_ip(&self, nym_address: &Recipient) -> Option<IpAddr> {
        self.clients.iter().find_map(|(ip, client)| {
            if client.nym_address == *nym_address {
                Some(*ip)
            } else {
                None
            }
        })
    }

    fn get_client_by_nym_address(&self, nym_address: &Recipient) -> Option<&ConnectedClient> {
        self.clients
            .values()
            .find(|client| client.nym_address == *nym_address)
    }

    fn connect(&mut self, ip: IpAddr, nym_address: Recipient, mix_hops: Option<u8>) {
        self.clients.insert(
            ip,
            ConnectedClient {
                nym_address,
                mix_hops,
                last_activity: std::time::Instant::now(),
            },
        );
        self.connected_client_tx
            .send(ConnectedClientEvent::Connect(Box::new(ConnectEvent {
                ip,
                nym_address,
                mix_hops,
            })))
            .unwrap();
    }

    fn update_activity(&mut self, ip: &IpAddr) -> Result<()> {
        if let Some(client) = self.clients.get_mut(ip) {
            client.last_activity = std::time::Instant::now();
            Ok(())
        } else {
            Err(IpPacketRouterError::FailedToUpdateClientActivity)
        }
    }

    fn disconnect_inactive_clients(&mut self) {
        let now = std::time::Instant::now();
        let inactive_clients: Vec<IpAddr> = self
            .clients
            .iter()
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
            self.clients.remove(&ip);
            self.connected_client_tx
                .send(ConnectedClientEvent::Disconnect(DisconnectEvent(ip)))
                .unwrap();
        }
    }

    fn find_new_ip(&self) -> Option<IpAddr> {
        generate_new_ip::find_new_ip(&self.clients)
    }
}

pub(crate) struct ConnectedClient {
    pub(crate) nym_address: Recipient,
    pub(crate) mix_hops: Option<u8>,
    pub(crate) last_activity: std::time::Instant,
}

#[cfg(target_os = "linux")]
impl MixnetListener {
    async fn on_static_connect_request(
        &mut self,
        connect_request: nym_ip_packet_requests::StaticConnectRequest,
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
                self.connected_clients
                    .connect(requested_ip, reply_to, reply_to_hops);
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

        if let Some(existing_ip) = self.connected_clients.get_ip(&reply_to) {
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

        self.connected_clients
            .connect(new_ip, reply_to, reply_to_hops);
        Ok(Some(IpPacketResponse::new_dynamic_connect_success(
            request_id, reply_to, new_ip,
        )))
    }

    async fn on_data_request(
        &mut self,
        data_request: nym_ip_packet_requests::DataRequest,
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

        // Check if there is a connected client for this src_addr. If there is, update the last activity time
        // for the client. If there isn't, drop the packet.
        if self.connected_clients.update_activity(&src_addr).is_err() {
            log::info!("Dropping packet: no connected client for {src_addr}");
            return Ok(None);
        }

        // Filter check
        let dst = dst.unwrap_or_else(|| SocketAddr::new(dst_addr, 0));
        if !self.request_filter.check_address(&dst).await {
            log::info!("Denied filter check: {dst}");
            // TODO: we could consider sending back a response here
            return Err(IpPacketRouterError::AddressFailedFilterCheck { addr: dst });
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
            .get_client_by_nym_address(recipient)
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
                    self.connected_clients.disconnect_inactive_clients();
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
    pub(crate) nym_address: Recipient,
    pub(crate) mix_hops: Option<u8>,
}
