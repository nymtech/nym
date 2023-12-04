use std::{collections::HashMap, net::IpAddr};

use futures::StreamExt;
use nym_ip_packet_requests::{
    DynamicConnectFailureReason, IpPacketRequest, IpPacketRequestData, IpPacketResponse,
    StaticConnectFailureReason,
};
use nym_sdk::mixnet::{InputMessage, MixnetMessageSender, Recipient};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::{connections::TransmissionLane, TaskHandle};
#[cfg(target_os = "linux")]
use tokio::io::AsyncWriteExt;

use crate::{
    constants::{CLIENT_INACTIVITY_TIMEOUT, DISCONNECT_TIMER_INTERVAL},
    error::IpPacketRouterError,
    request_filter::{self},
    util::generate_new_ip,
    util::parse_ip::{parse_packet, ParsedPacket},
    Config,
};

#[cfg(target_os = "linux")]
pub(crate) struct MixnetListener {
    pub(crate) _config: Config,
    pub(crate) request_filter: request_filter::RequestFilter,
    pub(crate) tun_writer: tokio::io::WriteHalf<tokio_tun::Tun>,
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,
    pub(crate) task_handle: TaskHandle,

    pub(crate) connected_clients: HashMap<IpAddr, ConnectedClient>,
    pub(crate) connected_client_tx: tokio::sync::mpsc::UnboundedSender<ConnectedClientEvent>,
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
    ) -> Result<Option<IpPacketResponse>, IpPacketRouterError> {
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
                        mix_hops: reply_to_hops,
                        last_activity: std::time::Instant::now(),
                    },
                );
                self.connected_client_tx
                    .send(ConnectedClientEvent::Connect(ConnectEvent {
                        ip: requested_ip,
                        nym_address: reply_to,
                        mix_hops: reply_to_hops,
                    }))
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
        let reply_to_hops = connect_request.reply_to_hops;
        // TODO: ignoring reply_to_avg_mix_delays for now

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
                mix_hops: reply_to_hops,
                last_activity: std::time::Instant::now(),
            },
        );
        self.connected_client_tx
            .send(ConnectedClientEvent::Connect(ConnectEvent {
                ip: new_ip,
                nym_address: reply_to,
                mix_hops: reply_to_hops,
            }))
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

    pub(crate) async fn run(mut self) -> Result<(), IpPacketRouterError> {
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
                        self.connected_client_tx.send(ConnectedClientEvent::Disconnect(DisconnectEvent(ip))).unwrap();
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
    Disconnect(DisconnectEvent),
    Connect(ConnectEvent),
}

pub(crate) struct DisconnectEvent(pub(crate) IpAddr);

pub(crate) struct ConnectEvent {
    pub(crate) ip: IpAddr,
    pub(crate) nym_address: Recipient,
    pub(crate) mix_hops: Option<u8>,
}
